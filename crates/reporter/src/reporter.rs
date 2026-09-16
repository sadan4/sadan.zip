use crate::{
	diag::ReporterError,
	fetcher::ScrapedOutput,
	util::{MultiProgressWrapper, Stage},
	vc::Plugin,
};
use ast_parser::pool::AllocPool;
use dashmap::DashMap;
use derive_more::IsVariant;
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use itertools::Itertools as _;
use miette::{Diagnostic, Severity};
use oxc::semantic::Stats;
use patch_engine::{
	ApplyOptions,
	SyntaxErrorReport,
	apply_patch,
	matches_module,
};
use rayon::iter::{
	IntoParallelIterator,
	IntoParallelRefIterator,
	ParallelIterator,
};
use smol_str::format_smolstr;
use std::{
	collections::{HashMap, HashSet},
	mem,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::{
	sync::{mpsc, oneshot},
	task,
};
use tracing::{debug, error};
use vencord_ast_parser::{Match, Patch};

#[derive(Debug)]
pub enum Msg {
	RequestProgressBar(oneshot::Sender<MultiProgressWrapper>),
	Error(ReporterError),
	Done(Duration),
}

impl From<ReporterError> for Msg {
	fn from(v: ReporterError) -> Self {
		Self::Error(v)
	}
}

pub fn report_broken_patches(
	channel: Channel,
	target_build: Arc<ScrapedOutput>,
	plugins: Arc<Vec<Plugin>>,
) -> mpsc::Receiver<Msg> {
	const BUFFER_SIZE: usize = 0x4000;
	let (tx, rx) = mpsc::channel(BUFFER_SIZE);
	let handle = task::spawn_blocking(move || {
		let start = Instant::now();
		run_reporter(channel, &target_build, &plugins, &tx);
		let duration = start.elapsed();
		tx.blocking_send(Msg::Done(duration))
			.unwrap();
	});

	task::spawn(async move {
		if let Err(e) = handle.await {
			error!("Reporter thread panicked: {e:?}");
		} else {
			debug!("Reporter thread finished successfully");
		}
	});

	rx
}

pub(crate) struct ReporterState<'a> {
	pub(crate) tx: &'a mpsc::Sender<Msg>,
	pub(crate) m_bar: MultiProgressWrapper,
	pub(crate) patches: HashSet<&'a Patch>,
	pub(crate) find_map: HashMap<&'a Patch, Vec<ModuleId>>,
	pub(crate) alloc: AllocPool,
	pub(crate) build: &'a ScrapedOutput,
	pub(crate) stats: DashMap<ModuleId, Stats>,
	pub(crate) channel: Channel,
}

#[derive(Copy, Clone, IsVariant)]
pub enum PatchStatus {
	Ok,
	Error,
}

impl<'a> ReporterState<'a> {
	fn new(
		plugins: &'a [Plugin],
		build: &'a ScrapedOutput,
		tx: &'a mpsc::Sender<Msg>,
		channel: Channel,
	) -> Self {
		let (pb_tx, rx) = oneshot::channel();
		tx.blocking_send(Msg::RequestProgressBar(pb_tx))
			.unwrap();
		let patches: HashSet<&Patch> = plugins
			.iter()
			.flat_map(|p| p.patches.iter())
			.collect();
		let stats = DashMap::with_capacity(build.len());
		let mut find_map: HashMap<_, _> = patches
			.iter()
			.map(|&p| (p, Vec::new()))
			.collect();
		find_map.shrink_to_fit();
		let m_bar = rx.blocking_recv().unwrap();
		Self {
			tx,
			m_bar,
			build,
			patches,
			stats,
			find_map,
			alloc: AllocPool::new(None),
			channel,
		}
	}

	fn run(mut self) {
		let start_time = Instant::now();
		let mut last = start_time;
		self.prune_bad_finds();
		let prune_time = last.elapsed();
		last = Instant::now();
		self.collect_finds();
		let collect_time = last.elapsed();
		last = Instant::now();
		self.report_empty_finds();
		let report_empty_time = last.elapsed();
		last = Instant::now();
		self.resolve_ambiguous_finds();
		let resolve_time = last.elapsed();
		last = Instant::now();
		self.test_patches();
		let test_time = last.elapsed();
		debug!(
			"Reporter finished in {total:.2?} (prune: {prune:.2?}, collect: {collect:.2?}, report_empty: {report_empty:.2?}, resolve: {resolve:.2?}, test: {test:.2?})",
			total = start_time.elapsed(),
			prune = prune_time,
			collect = collect_time,
			report_empty = report_empty_time,
			resolve = resolve_time,
			test = test_time,
		);
	}
	#[must_use = "RAII guard"]
	fn stage(&self, msg: &'static str, n: Option<usize>) -> Stage {
		Stage::new(format!("[{:?}]: {msg}", self.channel), n)
			.and_attach(&self.m_bar)
	}
	pub(crate) fn prune_bad_finds(&mut self) {
		let bar = self.stage("Pruning bad finds", Some(self.patches.len()));
		self.patches.retain(|p| {
			bar.step();
			if let Match::Regex(r) = &p.find.v
				&& let Err(e) = r.regex()
			{
				self.tx
					.blocking_send(
						ReporterError::BadRegexSyntax {
							plugin_id: p.plugin_id(),
							source: e.clone(),
							regex_span: p.find.s.into(),
							expanded: format!("/{}/{}", r.pattern, r.flags),
						}
						.into(),
					)
					.unwrap();
				false
			} else {
				true
			}
		});
	}
	pub(crate) fn collect_finds(&mut self) {
		let progress =
			self.stage("Collecting find matches", Some(self.patches.len()));
		self.find_map = self
			.patches
			.par_iter()
			.map(|patch| {
				let matches = self
					.build
					.par_iter()
					.filter_map(|(m_id, m_txt)| {
						if matches_module(m_txt, patch)
							.expect("prune_bad_finds should be called first")
						{
							Some(*m_id)
						} else {
							None
						}
					})
					.collect();
				progress.step();
				(*patch, matches)
			})
			.collect();
	}
	pub(crate) fn report_empty_finds(&mut self) {
		_ = self.stage("Reporting empty finds", None);
		for (patch, _) in self
			.find_map
			.extract_if(|_, patch| patch.is_empty())
		{
			let mut err = ReporterError::FindNotFound {
				find_span: patch.find.s.into(),
				plugin_id: patch.plugin_id(),
				patch_hash: patch.content_hash(),
			};
			if patch.no_warn {
				err = ReporterError::NoWarn(Box::new(err));
			}
			self.tx
				.blocking_send(err.into())
				.unwrap();
		}
	}
	pub(crate) fn resolve_ambiguous_finds(&mut self) {
		let it = self
			.find_map
			.extract_if(|p, m| !p.all && m.len() > 1)
			.collect_vec();
		let bar = self.stage("Resolving ambiguous finds", Some(it.len()));
		it.into_par_iter().for_each(|(patch, matches)| {
			let mut failed = Vec::new();
			let mut good = Vec::new();
			matches.iter().copied().for_each(|m_id| {
				match self.test_patch_against_module(patch, m_id, None) {
					PatchStatus::Ok => good.push(m_id),
					PatchStatus::Error => failed.push(m_id),
				}
			});
			// TODO: suppress if patch is no_warn??
			let err = if good.len() == 1 {
				ReporterError::FindAmbiguousRecoverable {
					find_span: patch.find.s.into(),
					plugin_id: patch.plugin_id(),
					ok_id: good[0],
					extra_help: if failed.is_empty() {
						"\nIf you intended for this patch to apply to all of the above modules, add the `all` property to the patch."
					} else {
						Default::default()
					},
					err_ids: failed
						.into_iter()
						.map(u32::from)
						.collect(),
				}
			} else {
				ReporterError::FindAmbiguous {
					find_span: patch.find.s.into(),
					plugin_id: patch.plugin_id(),
					ok_ids: good
						.into_iter()
						.map(u32::from)
						.collect(),
					err_ids: failed
						.into_iter()
						.map(u32::from)
						.collect(),
				}
			};
			self.tx
				.blocking_send(err.into())
				.unwrap();
			bar.step();
		});
	}
	pub(crate) fn test_patches(&mut self) {
		// temporarily take the find_map so we don't have to deal with 2x &mut self
		let found_patches = mem::take(&mut self.find_map);
		let bar = self.stage("Testing patches", Some(found_patches.len()));
		found_patches
			.par_iter()
			.for_each(|(patch, ids)| {
				ids.into_par_iter()
					.fold(Vec::new, |mut errs, &m_id| {
						self.test_patch_against_module(
							patch,
							m_id,
							Some(&mut errs),
						);
						errs
					})
					.flatten()
					.for_each(|err| {
						self.tx
							.blocking_send(err.into())
							.unwrap();
					});
				bar.step();
			});
		self.find_map = found_patches;
	}

	pub(crate) fn test_patch_against_module(
		&self,
		patch: &'a Patch,
		m_id: ModuleId,
		mut errs: Option<&mut Vec<ReporterError>>,
	) -> PatchStatus {
		let m_txt = self
			.build
			.get(&m_id)
			.expect("invalid module id");
		let file_name = format_smolstr!("{m_id}.js");
		let mut alloc = self.alloc.get();
		let applied = apply_patch(
			&mut alloc,
			patch,
			format!("0,{m_txt}"),
			&ApplyOptions {
				syntax_errors: SyntaxErrorReport::Formatted {
					file_name: Some(&file_name),
				},
				stats: self.stats.get(&m_id).map(|x| *x),
				plugin_name: Some("MyPlugin"),
				..ApplyOptions::default()
			},
		);

		if let Some(stats) = applied.stats {
			self.stats.entry(m_id).or_insert(stats);
		}

		let mut status = PatchStatus::Ok;
		for event in applied.events {
			let e = ReporterError::from_event(event, patch.plugin_id(), m_id);
			if !e.is_no_warn()
				&& e.severity()
					.is_none_or(|s| s == Severity::Error)
			{
				status = PatchStatus::Error;
			}
			if let Some(errs) = &mut errs {
				errs.push(e);
			}
		}
		status
	}
}

fn run_reporter(
	channel: Channel,
	build: &ScrapedOutput,
	plugins: &[Plugin],
	tx: &mpsc::Sender<Msg>,
) {
	ReporterState::new(plugins, build, tx, channel).run();
}
