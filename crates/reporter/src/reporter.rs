use crate::{
	diag::ReporterError,
	fetcher::ScrapedOutput,
	util::{MultiProgressWrapper, Stage},
	vc::Plugin,
};
use anyhow::{Context, Result, anyhow};
use derive_more::IsVariant;
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use itertools::{Itertools as _, PutBack, put_back};
use miette::{Diagnostic, NamedSource, Severity, SourceCode};
use oxc::{
	allocator::Allocator,
	ast::ast::RegExpFlags,
	diagnostics::OxcDiagnostic,
	parser::Parser,
	semantic::{SemanticBuilder, Stats},
	span::{SourceType, Span},
};
use pretty_printer::{FormattedContent, format_with_alloc};
use regress::Regex;
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
use tracing::error;
use vencord_ast_parser::{Match, Patch, Replacement, Replacer};

#[derive(Debug)]
pub enum Msg {
	RequestProgressBar(oneshot::Sender<&'static MultiProgressWrapper>),
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
	channel: Channel,
	target_build: Arc<ScrapedOutput>,
	plugins: Arc<Vec<Plugin>>,
) -> mpsc::Receiver<Msg> {
	const BUFFER_SIZE: usize = 0x4000;
	let (mut tx, rx) = mpsc::channel(BUFFER_SIZE);
	task::spawn_blocking(move || {
		let start = Instant::now();
		run_reporter(channel, &target_build, &plugins, &mut tx);
		let duration = start.elapsed();
		tx.blocking_send(Msg::Done(Ok(duration)))
			.unwrap();
	});

	rx
}

struct ReporterState<'a> {
	tx: &'a mut mpsc::Sender<Msg>,
	m_bar: &'static MultiProgressWrapper,
	patches: HashSet<&'a Patch>,
	find_map: HashMap<&'a Patch, Vec<ModuleId>>,
	alloc: Allocator,
	build: &'a ScrapedOutput,
	stats: HashMap<ModuleId, Stats>,
	channel: Channel,
}

impl<'a> ReporterState<'a> {
	fn new(
		plugins: &'a [Plugin],
		build: &'a ScrapedOutput,
		tx: &'a mut mpsc::Sender<Msg>,
		channel: Channel,
	) -> Self {
		let (pb_tx, rx) = oneshot::channel();
		tx.blocking_send(Msg::RequestProgressBar(pb_tx))
			.unwrap();
		let patches: HashSet<&Patch> = plugins
			.iter()
			.flat_map(|p| p.patches.iter())
			.collect();
		let stats = HashMap::with_capacity(build.len());
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
			channel,
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
		Stage::new(format!("[{:?}]: {msg}", self.channel), n)
			.and_attach(self.m_bar)
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
		let progress =
			self.stage("Collecting find matches", Some(self.build.len()));
		for (&m_id, m_txt) in self.build {
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
					cause: formatted_error,
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
	) -> Result<Box<dyn Diagnostic + Send + Sync + 'static>> {
		let FormattedContent {
			code: mut formatted_source,
			mappings,
		} = format_with_alloc(original_source, &self.alloc, 2)
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
		if let Some(labels) = &mut e.labels {
			for label in labels {
				// miette is evil and doesn't let you mutate the offset or go into string
				let label_span = Span::new(
					label.offset() as u32,
					(label.offset() + label.len()) as u32,
				);
				// i can't get Option.cloned() to work for some reason
				let txt = label.label().map(String::from);
				*label = Self::find_new_span(&mappings, label_span).into();
				label.set_label(txt);
			}
		}

		let src = NamedSource::new(format!("{m_id}.js"), formatted_source)
			.with_language("JavaScript");
		Ok(Box::new(WrappedOxcDiagnostic::new(e, src)))
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
		&mut self,
		new_src: &str,
		m_id: ModuleId,
	) -> Result<(), OxcDiagnostic> {
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
	channel: Channel,
	build: &ScrapedOutput,
	plugins: &[Plugin],
	tx: &mut mpsc::Sender<Msg>,
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
	if !p_ret.errors.is_empty() {
		let ret = p_ret.errors.swap_remove(0);
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
	if sema.errors.is_empty() {
		Ok(sema.semantic.stats())
	} else {
		let ret = sema.errors.swap_remove(0);
		Err(ret)
	}
}

struct WrappedOxcDiagnostic {
	diag: OxcDiagnostic,
	src: Box<dyn SourceCode>,
}
impl WrappedOxcDiagnostic {
	fn new(diag: OxcDiagnostic, src: NamedSource<String>) -> Self {
		Self {
			diag,
			src: Box::new(src),
		}
	}
}

impl std::fmt::Display for WrappedOxcDiagnostic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		<OxcDiagnostic as std::fmt::Display>::fmt(&self.diag, f)
	}
}

impl std::fmt::Debug for WrappedOxcDiagnostic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		<OxcDiagnostic as std::fmt::Debug>::fmt(&self.diag, f)
	}
}

impl std::error::Error for WrappedOxcDiagnostic {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		self.diag.source()
	}

	fn description(&self) -> &str {
		#[allow(deprecated)]
		self.diag.description()
	}

	fn cause(&self) -> Option<&dyn std::error::Error> {
		#[allow(deprecated)]
		self.diag.cause()
	}
}

impl Diagnostic for WrappedOxcDiagnostic {
	fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.diag.code()
	}

	fn severity(&self) -> Option<Severity> {
		self.diag.severity()
	}

	fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.diag.help()
	}

	fn note<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.diag.note()
	}

	fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.diag.url()
	}

	fn source_code(&self) -> Option<&dyn SourceCode> {
		Some(self.src.as_ref())
	}

	fn labels(
		&self,
	) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
		self.diag.labels()
	}

	fn related<'a>(
		&'a self,
	) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
		self.diag.related()
	}

	fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
		self.diag.diagnostic_source()
	}
}
