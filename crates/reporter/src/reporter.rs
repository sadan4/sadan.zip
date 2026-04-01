use std::{
	collections::{HashMap, HashSet},
	mem,
	sync::Arc,
	time::{Duration, Instant},
};

use crate::{
	diag::ReporterError,
	util::Stage,
	vc::{Match, Patch, Plugin, Replacer},
};
use anyhow::{Result, anyhow};
use derive_more::IsVariant;
use explorer_types::FullBundle;
use indicatif::MultiProgress;
use itertools::Itertools as _;
use miette::{Diagnostic, Severity};
use oxc::{
	allocator::Allocator,
	ast::ast::RegExpFlags,
	parser::Parser,
	semantic::{SemanticBuilder, Stats},
	span::SourceType,
};
use tokio::{
	sync::{mpsc, oneshot},
	task,
};

#[derive(Debug)]
pub enum Msg {
	RequestProgressBar(oneshot::Sender<MultiProgress>),
	Error(ReporterError),
	Done(Result<Duration>),
}

impl From<ReporterError> for Msg {
	fn from(v: ReporterError) -> Self {
		Self::Error(v)
	}
}

#[track_caller]
pub fn report_broken_patches(
	target_build: Arc<FullBundle>,
	plugins: Arc<Vec<Plugin>>,
) -> mpsc::Receiver<Msg> {
	const BUFFER_SIZE: usize = 0x4000;
	let (mut tx, rx) = mpsc::channel(BUFFER_SIZE);
	task::spawn_blocking(move || {
		let start = Instant::now();
		run_reporter(&target_build, &plugins, &mut tx);
		let duration = start.elapsed();
		tx.blocking_send(Msg::Done(Ok(duration)))
			.unwrap();
	});

	rx
}

struct ReporterState<'a> {
	tx: &'a mut mpsc::Sender<Msg>,
	m_bar: MultiProgress,
	patches: HashSet<&'a Patch>,
	find_map: HashMap<&'a Patch, Vec<u32>>,
	alloc: Allocator,
	build: &'a FullBundle,
	stats: HashMap<u32, Stats>,
}

impl<'a> ReporterState<'a> {
	fn new(
		plugins: &'a [Plugin],
		build: &'a FullBundle,
		tx: &'a mut mpsc::Sender<Msg>,
	) -> Self {
		let (pb_tx, rx) = oneshot::channel();
		tx.blocking_send(Msg::RequestProgressBar(pb_tx))
			.unwrap();
		let patches: HashSet<&Patch> = plugins
			.iter()
			.flat_map(|p| p.patches.iter())
			.collect();
		let stats = HashMap::with_capacity(build.modules.len());
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
			alloc: Allocator::new(),
		}
	}
}

#[derive(Copy, Clone, IsVariant)]
enum PatchStatus {
	Ok,
	Error,
}

#[allow(clippy::multiple_inherent_impl)]
impl<'a> ReporterState<'a> {
	fn run(mut self) {
		self.prune_bad_finds();
		self.collect_finds();
		self.report_empty_finds();
		self.resolve_ambiguous_finds();
		self.test_patches();
	}
	#[must_use = "RAII guard"]
	fn stage(&self, msg: &'static str, n: Option<usize>) -> Stage {
		Stage::new(msg, n).and_attach(&self.m_bar)
	}
	fn prune_bad_finds(&mut self) {
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
							source: anyhow!("{e:?}"),
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
	fn collect_finds(&mut self) {
		let progress = self
			.stage("Collecting find matches", Some(self.build.modules.len()));
		for (&m_id, m_txt) in &self.build.modules {
			for patch in &self.patches {
				if matches_module(m_txt, patch) {
					// this should never error because we pre-fill all the keys with empty vectors in the ctor
					self.find_map
						.get_mut(patch)
						.unwrap()
						.push(m_id);
				}
			}
			progress.step();
		}
	}
	fn report_empty_finds(&mut self) {
		_ = self.stage("Reporting empty finds", None);
		for (patch, _) in self
			.find_map
			.extract_if(|_, patch| patch.is_empty())
		{
			let mut err = ReporterError::FindNotFound {
				find_span: patch.find.s.into(),
				plugin_id: patch.plugin_id(),
			};
			if patch.no_warn {
				err = ReporterError::NoWarn(err.into());
			}
			self.tx
				.blocking_send(err.into())
				.unwrap();
		}
	}
	fn resolve_ambiguous_finds(&mut self) {
		let it = self
			.find_map
			.extract_if(|p, m| !p.all && m.len() > 1)
			.collect_vec();
		let bar = self.stage("Resolving ambiguous finds", Some(it.len()));
		for (patch, matches) in it {
			let mut failed = Vec::new();
			let mut good = Vec::new();
			for m_id in matches.iter().copied() {
				match self.test_patch_against_module(patch, m_id, None) {
					PatchStatus::Ok => good.push(m_id),
					PatchStatus::Error => failed.push(m_id),
				}
			}
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
					err_ids: failed,
				}
			} else {
				ReporterError::FindAmbiguous {
					find_span: patch.find.s.into(),
					plugin_id: patch.plugin_id(),
					ok_ids: good,
					err_ids: failed,
				}
			};
			self.tx
				.blocking_send(err.into())
				.unwrap();
			bar.step();
		}
	}
	fn test_patches(&mut self) {
		// temporarily take the find_map so we don't have to deal with 2x &mut self
		let found_patches = mem::take(&mut self.find_map);
		let bar = self.stage("Testing patches", Some(found_patches.len()));
		let mut errs = Vec::new();
		for (patch, ids) in &found_patches {
			for &m_id in ids {
				self.test_patch_against_module(patch, m_id, Some(&mut errs));
			}
			for err in errs.drain(..) {
				self.tx
					.blocking_send(err.into())
					.unwrap();
			}
			bar.step();
		}
		self.find_map = found_patches;
	}
	fn test_patch_against_module(
		&mut self,
		patch: &'a Patch,
		m_id: u32,
		mut errs: Option<&mut Vec<ReporterError>>,
	) -> PatchStatus {
		let mut status = PatchStatus::Ok;
		let m_txt = self
			.build
			.modules
			.get(&m_id)
			.expect("invalid module id");
		let mut last_src = format!("0,{m_txt}");
		let plugin_id = patch.plugin_id();
		let mut report = |e: ReporterError| {
			if !e.is_no_warn()
				&& e.severity()
					.is_none_or(|s| s == Severity::Error)
			{
				status = PatchStatus::Error;
			}
			if let Some(errs) = &mut errs {
				errs.push(e);
			}
		};

		for r in &patch.replacement {
			let no_warn = patch.no_warn || r.no_warn;
			let is_global = r
				.match_
				.v
				.as_regex()
				.is_some_and(|r| r.flags.contains(RegExpFlags::G));

			let Some(pat) =
				Self::compile_replacement_pattern(r, plugin_id, &mut report)
			else {
				continue;
			};

			if !Self::validate_match_occurrence(
				pat,
				&last_src,
				is_global,
				no_warn,
				r,
				m_id,
				plugin_id,
				&mut report,
			) {
				continue;
			}

			let new_src = Self::apply_replacement(
				pat,
				&last_src,
				&r.replace.v,
				is_global,
			);

			if let Err(e) = self.check_and_update_syntax(&new_src, m_id) {
				report(ReporterError::ReplaceSyntaxError {
					replace_span: r.replace.s.into(),
					cause: e,
					module_id: m_id,
					plugin_id,
				});
			}

			last_src = new_src;
		}
		status
	}

	fn compile_replacement_pattern<'r>(
		replacement: &'r crate::vc::Replacement,
		plugin_id: u16,
		report: &mut impl FnMut(ReporterError),
	) -> Option<&'r regress::Regex> {
		match &replacement.match_.v {
			Match::Str(_) => {
				unreachable!()
			}
			Match::Regex(v) => match v.regex() {
				Ok(r) => Some(r),
				Err(e) => {
					report(ReporterError::BadRegexSyntax {
						plugin_id,
						source: anyhow!("{e:?}"),
						regex_span: replacement.match_.s.into(),
						expanded: format!("/{}/{}", v.pattern, v.flags),
					});
					None
				}
			},
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn validate_match_occurrence(
		pat: &regress::Regex,
		src: &str,
		is_global: bool,
		no_warn: bool,
		replacement: &crate::vc::Replacement,
		m_id: u32,
		plugin_id: u16,
		report: &mut impl FnMut(ReporterError),
	) -> bool {
		let mut it = pat.find_iter(src);

		if it.next().is_none() {
			let mut err = ReporterError::ReplaceMatchNotFound {
				match_span: replacement.match_.s.into(),
				module_id: m_id,
				plugin_id,
			};
			if no_warn {
				err = ReporterError::NoWarn(Box::new(err));
			}
			report(err);
			return false;
		}

		if !is_global && it.next().is_some() {
			report(ReporterError::ReplaceMatchAmbiguous {
				match_span: replacement.match_.s.into(),
				plugin_id,
				module_id: m_id,
			});
		}

		true
	}

	fn apply_replacement(
		pat: &regress::Regex,
		src: &str,
		replacer: &Replacer,
		is_global: bool,
	) -> String {
		match replacer {
			Replacer::Str(s) => {
				if is_global {
					pat.replace_all(src, s.as_str())
				} else {
					pat.replace(src, s.as_str())
				}
			}
			Replacer::Template(e) => {
				if is_global {
					pat.replace_all_with(src, e.make_replacer(src))
				} else {
					pat.replace_with(src, e.make_replacer(src))
				}
			}
		}
	}

	fn check_and_update_syntax(
		&mut self,
		new_src: &str,
		m_id: u32,
	) -> Result<(), Box<dyn Diagnostic + Send + Sync>> {
		let result = {
			let chk = check_syntax_errors(
				&self.alloc,
				new_src,
				self.stats.get(&m_id).copied(),
			);
			self.alloc.reset();
			chk
		};

		match result {
			Ok(stats) => {
				self.stats.entry(m_id).or_insert(stats);
				Ok(())
			}
			Err(e) => Err(e),
		}
	}
}

fn run_reporter(
	build: &FullBundle,
	plugins: &[Plugin],
	tx: &mut mpsc::Sender<Msg>,
) {
	ReporterState::new(plugins, build, tx).run();
}

fn matches_module(m_txt: &str, patch: &Patch) -> bool {
	match &patch.find.v {
		Match::Str(s) => s.find(m_txt.as_bytes()).is_some(),
		Match::Regex(s) => {
			// we should never have a patch with bad regex
			// it should have been filtered out
			s.regex()
				.as_ref()
				.unwrap()
				.find(m_txt)
				.is_some()
		}
	}
}

fn check_syntax_errors(
	alloc: &Allocator,
	src: &str,
	stats: Option<Stats>,
) -> Result<Stats, Box<dyn Diagnostic + Send + Sync>> {
	let mut p_ret = Parser::new(alloc, src, SourceType::unambiguous()).parse();
	if !p_ret.errors.is_empty() {
		let ret = p_ret.errors.swap_remove(0);
		return Err(ret.into());
	}
	let sema = SemanticBuilder::new()
		.with_check_syntax_error(true)
		.with_cfg(false);
	let sema = if let Some(stats) = stats {
		sema.with_stats(stats)
	} else {
		sema
	};
	let mut sema = sema.build(&p_ret.program);
	if sema.errors.is_empty() {
		Ok(sema.semantic.stats())
	} else {
		let ret = sema.errors.swap_remove(0);
		Err(ret.into())
	}
}
