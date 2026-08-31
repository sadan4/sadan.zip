use crate::{
	diag::ReporterError,
	fetcher::ScrapedOutput,
	util::{MultiProgressWrapper, Stage},
	vc::Plugin,
};
use anyhow::{Context, Result};
use ast_parser::diag::{SourceCode, WrappedOxcDiagnostic};
use dashmap::DashMap;
use derive_more::IsVariant;
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use itertools::{Itertools as _, PutBack, put_back};
use miette::{Diagnostic, Severity};
use oxc::{
	allocator::Allocator,
	ast::ast::RegExpFlags,
	diagnostics::OxcDiagnostic,
	parser::Parser,
	semantic::{SemanticBuilder, Stats},
	span::{SourceType, Span},
};
use oxc_allocator::AllocatorPool;
use pretty_printer::{FormattedContent, format_with_alloc};
use rayon::iter::{
	IntoParallelIterator,
	IntoParallelRefIterator,
	ParallelIterator,
};
use regress::Regex;
use smol_str::{SmolStr, format_smolstr};
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
use vencord_ast_parser::{Match, Patch, Replacement, Replacer};

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
	pub(crate) alloc: AllocatorPool,
	pub(crate) build: &'a ScrapedOutput,
	pub(crate) stats: DashMap<ModuleId, Stats>,
	pub(crate) channel: Channel,
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
			alloc: AllocatorPool::new(num_cpus::get()),
			channel,
		}
	}
}

#[derive(Copy, Clone, IsVariant)]
pub enum PatchStatus {
	Ok,
	Error,
}

impl<'a> ReporterState<'a> {
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
						if matches_module(m_txt, patch) {
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
		let mut status = PatchStatus::Ok;
		let m_txt = self
			.build
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
				let formatted_error = match self.format_syntax_error(
					e,
					&last_src,
					m_id,
					pat,
					&r.replace.v,
					is_global,
				) {
					Ok(e) => e,
					Err(e) => {
						error!(
							"Failed to format syntax error, skipping: {e:?}"
						);
						continue;
					}
				};
				report(ReporterError::ReplaceSyntaxError {
					replace_span: r.replace.s.into(),
					cause: Box::new(formatted_error),
					module_id: m_id,
					plugin_id,
				});
			}

			last_src = new_src;
		}
		status
	}

	fn format_syntax_error(
		&self,
		mut e: OxcDiagnostic,
		original_source: &str,
		m_id: ModuleId,
		pat: &Regex,
		replacement: &Replacer,
		is_global: bool,
	) -> Result<WrappedOxcDiagnostic> {
		let alloc = self.alloc.get();
		let FormattedContent {
			code: mut formatted_source,
			mappings,
		} = format_with_alloc(original_source, &alloc, 2)
			.context("Failed to format valid module source")?;
		// determine the ranges and contents of each replacement
		let mut ranges = Vec::new();
		if is_global {
			for m in pat.find_iter(original_source) {
				let repl_txt = replacement.do_replace(original_source, &m);
				ranges.push((
					Span::new(m.start() as u32, m.end() as u32),
					repl_txt,
				));
			}
			debug_assert!(
				!ranges.is_empty(),
				"we should only be here if a previous replacement applied with a syntax error"
			);
		} else {
			let m = pat.find(original_source).unwrap();
			let repl_txt = replacement.do_replace(original_source, &m);
			ranges
				.push((Span::new(m.start() as u32, m.end() as u32), repl_txt));
		}
		let mut new_ranges = Vec::with_capacity(ranges.len());

		for (before_span, replaced_text) in ranges {
			let new_range = Self::find_new_span(&mappings, before_span);
			new_ranges.push((new_range, replaced_text));
		}

		// TODO: reserve space for the replacements using data from previous loops
		// Do the replace and get the new source
		// sort so we can pop to iterate in reverse order
		new_ranges.sort_by_key(|a| a.0);
		// Iterate in reverse (pop()) so that the early ranges dont shift the later ones
		while let Some((repl_range, repl_txt)) = new_ranges.pop() {
			formatted_source.replace_range(
				repl_range.start as usize..repl_range.end as usize,
				&repl_txt,
			);
		}

		// Map the error spans from the original diagnostic
		for label in e.labels.as_mut_slice() {
			// miette is evil and doesn't let you mutate the offset or go into string
			let label_span =
				Span::new(label.offset(), label.offset() + label.len());
			// i can't get Option.cloned() to work for some reason
			let txt = label.label().map(String::from);
			let primary = label.primary();
			let new_span = Self::find_new_span(&mappings, label_span);
			let new_label = if primary {
				oxc::span::LabeledSpan::new_primary_with_span(txt, new_span)
			} else {
				oxc::span::LabeledSpan::new_with_span(txt, new_span)
			};
			*label = new_label
		}
		let mut ret = WrappedOxcDiagnostic::from(e);
		ret.source = Some(SourceCode {
			source_code: Arc::from(formatted_source),
			file_name: Some(format_smolstr!("{m_id}.js")),
			file_type: Some(SmolStr::new_static("JavaScript")),
		});
		Ok(ret)
	}

	fn find_new_span(mappings: &[(u32, u32)], original_span: Span) -> Span {
		let mut it = put_back(mappings.iter().copied().rev());
		let new_end = Self::find_new_pos(&mut it, original_span.end);
		let new_start = Self::find_new_pos(&mut it, original_span.start);
		Span::new(new_start, new_end)
	}

	/// Mappings must be a reverse iterator (highest to lowest)
	fn find_new_pos(
		mappings: &mut PutBack<impl Iterator<Item = (u32, u32)>>,
		prev: u32,
	) -> u32 {
		for (before, after) in mappings.by_ref() {
			if prev >= before {
				// We need to put it back because the next one might be within this mapping as well
				mappings.put_back((before, after));
				return after + (prev - before);
			}
		}
		unreachable!(
			"we should always find a mapping because the first mapping should be (0, 0)"
		)
	}

	fn compile_replacement_pattern<'r>(
		replacement: &'r Replacement,
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
						source: e.clone(),
						regex_span: replacement.match_.s.into(),
						expanded: format!("/{}/{}", v.pattern, v.flags),
					});
					None
				}
			},
		}
	}

	#[expect(clippy::too_many_arguments)]
	fn validate_match_occurrence(
		pat: &regress::Regex,
		src: &str,
		is_global: bool,
		no_warn: bool,
		replacement: &Replacement,
		m_id: ModuleId,
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
		&self,
		new_src: &str,
		m_id: ModuleId,
	) -> Result<(), OxcDiagnostic> {
		let alloc = self.alloc.get();
		let result = check_syntax_errors(
			&alloc,
			new_src,
			self.stats.get(&m_id).map(|x| *x),
		);

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
	channel: Channel,
	build: &ScrapedOutput,
	plugins: &[Plugin],
	tx: &mpsc::Sender<Msg>,
) {
	ReporterState::new(plugins, build, tx, channel).run();
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
) -> Result<Stats, OxcDiagnostic> {
	let mut p_ret = Parser::new(alloc, src, SourceType::unambiguous()).parse();
	if !p_ret.diagnostics.is_empty() {
		let ret = p_ret.diagnostics.swap_remove(0);
		return Err(ret);
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
	if sema.diagnostics.is_empty() {
		Ok(sema.semantic.stats())
	} else {
		let ret = sema.diagnostics.swap_remove(0);
		Err(ret)
	}
}
