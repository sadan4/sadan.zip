use ast_parser::diag::{OxcSourceSpan, WrappedOxcDiagnostic};
use miette::Diagnostic;
use oxc::{
	allocator::Allocator,
	ast::ast::RegExpFlags,
	diagnostics::OxcDiagnostic,
	parser::Parser,
	semantic::{SemanticBuilder, Stats},
	span::{SourceType, Span},
};
use regress::Regex;
use thiserror::Error;
use tracing::warn;
use vencord_ast_parser::{Match, Patch, Replacement, Replacer};

use crate::fmt::format_syntax_error;

/// How the parse error from a replacement that produced invalid JS is
/// reported.
#[derive(Clone, Copy, Debug)]
pub enum SyntaxErrorReport<'a> {
	/// Hand the error back as oxc produced it, with its labels pointing into
	/// the minified module.
	Raw,
	/// Pretty print the patched module and move the labels onto it, attaching
	/// that text to the diagnostic as `file_name`. See
	/// [`syntax_error_in_formatted`](crate::syntax_error_in_formatted).
	Formatted { file_name: Option<&'a str> },
}

impl Default for SyntaxErrorReport<'_> {
	fn default() -> Self {
		Self::Formatted {
			file_name: Some("file.js"),
		}
	}
}

pub struct ApplyOptions<'a> {
	/// When `Some`, `$self` in a replacement expands to
	/// `Vencord.Plugins.plugins["<name>"]`, the rewrite Vencord's
	/// `canonicalizeReplace` does at runtime.
	pub plugin_name: Option<&'a str>,
	/// Whether a replacement whose result fails to parse is kept.
	///
	/// `true` keeps it, so the replacements after it run against the broken
	/// source and every consequence gets reported. `false` throws it away and
	/// carries the last source that parsed forward, which is what a live
	/// preview wants.
	pub keep_broken_replacements: bool,
	pub syntax_errors: SyntaxErrorReport<'a>,
	/// Semantic stats from a previous run over this module, to size the
	/// semantic builder. See [`Applied::stats`].
	pub stats: Option<Stats>,
}

impl Default for ApplyOptions<'_> {
	fn default() -> Self {
		Self {
			plugin_name: None,
			keep_broken_replacements: true,
			syntax_errors: SyntaxErrorReport::default(),
			stats: None,
		}
	}
}

#[derive(Debug)]
pub struct Applied {
	/// The patched source.
	pub src: String,
	/// What went wrong on the way there, in the order of the replacements that
	/// caused it.
	pub events: Vec<ApplyEvent>,
	/// The stats of the last source that parsed, to feed back in as
	/// [`ApplyOptions::stats`] the next time this module is patched.
	pub stats: Option<Stats>,
}

#[derive(Debug, Diagnostic, Error)]
pub enum ApplyEvent {
	#[error("Bad Regex Syntax")]
	#[diagnostic[
        code(bad_regex_syntax),
        severity(Error),
        help("The regex was expanded to {expanded}"),
    ]]
	BadRegex {
		/// the span of the regex in the source file it came from
		#[label("From this regex")]
		regex_span: OxcSourceSpan,
		#[source]
		source: regress::Error,
		/// the pattern as the parser expanded it, for the error message
		expanded: String,
	},
	#[error("Match Not Found")]
	#[diagnostic[
        code(replace::match_not_found),
        severity(Error),
    ]]
	MatchNotFound {
		#[label("Caused by this match")]
		match_span: OxcSourceSpan,
	},
	#[error("Replace Match Ambiguous")]
	#[diagnostic[
        code(replace::match_ambiguous),
        severity(Warning),
    ]]
	MatchAmbiguous {
		#[label("Caused by this match")]
		match_span: OxcSourceSpan,
	},
	#[error("Replace Syntax Error")]
	#[diagnostic[
        code(replace::syntax_error),
        severity(Error),
    ]]
	SyntaxError {
		#[label("Caused by this replacement")]
		replace_span: OxcSourceSpan,
		#[source]
		#[diagnostic_source]
		cause: Box<WrappedOxcDiagnostic>,
	},
	/// An [`ApplyEvent`] that was suppressed because of [`Patch::no_warn`] or [`Replacement::no_warn`].
	#[error(transparent)]
	NoWarn(Box<Self>),
}

impl ApplyEvent {
	/// Suppress this event by wrapping it in [`ApplyEvent::NoWarn`].
	fn suppress(self) -> Self {
		Self::NoWarn(Box::new(self))
	}
	/// [`suppresses`](Self::suppress) this event if `predicate` is true.
	fn suppress_if(self, predicate: bool) -> Self {
		if predicate { self.suppress() } else { self }
	}
}

/// Runs `patch`'s replacements over `src`, in order.
///
/// `patch` must have been through
/// [`compile_patch_regexes`](crate::compile_patch_regexes) first.
#[must_use = "check `Applied` for errors"]
pub fn apply_patch(
	alloc: &mut Allocator,
	patch: &Patch,
	src: String,
	opts: &ApplyOptions<'_>,
) -> Applied {
	let self_expr = opts
		.plugin_name
		.map(|name| format!("Vencord.Plugins.plugins[{name:?}]"));
	let mut cur = src;
	let mut events = Vec::new();
	let mut stats = opts.stats;

	for r in &patch.replacement {
		let no_warn = patch.no_warn || r.no_warn;
		let is_global = r
			.match_
			.v
			.as_regex()
			.is_some_and(|r| r.flags.contains(RegExpFlags::G));

		let Some(pat) = get_repl_regex(r, &mut events) else {
			continue;
		};

		match check_match_target(pat, &cur, is_global, no_warn, r, &mut events)
		{
			MatchTarget::Found => {}
			MatchTarget::NotFound => continue,
		}

		let rewrite = Rewrite::new(&r.replace.v, self_expr.as_deref());
		let new_src = rewrite.apply(pat, &cur, is_global);

		alloc.reset();
		match check_syntax_errors(alloc, &new_src, stats) {
			Ok(new_stats) => stats = Some(new_stats.increase_by(0.1)),
			Err(e) => {
				events.push(ApplyEvent::SyntaxError {
					replace_span: r.replace.s.into(),
					cause: Box::new(report_syntax_error(
						alloc,
						e,
						&cur,
						pat,
						&rewrite,
						is_global,
						opts.syntax_errors,
					)),
				});
				if !opts.keep_broken_replacements {
					continue;
				}
			}
		}

		cur = new_src;
	}

	Applied {
		src: cur,
		events,
		stats,
	}
}

/// Whether `patch`'s `find` selects the module `src`.
pub fn matches_module<'a>(
	src: &str,
	patch: &'a Patch,
) -> Result<bool, &'a regress::Error> {
	Ok(match &patch.find.v {
		Match::Str(s) => s.find(src.as_bytes()).is_some(),
		Match::Regex(r) => r.regex().as_ref()?.find(src).is_some(),
	})
}

/// Parses `src`, returning the semantic stats to reuse for the next parse of
/// the same module, or the first error it has.
pub fn check_syntax_errors(
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

/// A `replace:` and the `$self` expansion that goes with it.
///
/// In a string replacement `$self` is expanded before `regress` sees the
/// string, so the `$` escapes in the rest of it keep working. In a template
/// replacement it is expanded in the text the template produces, which is what
/// Vencord's `canonicalizeReplace` does for replace functions.
// TODO: hoist to free functions?
struct Rewrite<'a> {
	replacer: &'a Replacer,
	/// Expression to expand `$self` into, if any.
	///
	/// If `None`, `$self` is not expanded.
	plugin_name: Option<&'a str>,
}

impl<'a> Rewrite<'a> {
	const fn new(replacer: &'a Replacer, plugin_name: Option<&'a str>) -> Self {
		Self {
			replacer,
			plugin_name,
		}
	}

	/// The text `m` is replaced with.
	fn text(&self, src: &str, m: &regress::Match) -> String {
		self.replacer
			.do_replace_with_self(src, m, self.plugin_name)
	}

	fn apply(&self, pat: &Regex, src: &str, is_global: bool) -> String {
		let f = |m: &regress::Match| self.text(src, m);
		if is_global {
			pat.replace_all_with(src, f)
		} else {
			pat.replace_with(src, f)
		}
	}

	/// Every span `pat` replaces in `src` and the text it is replaced with.
	fn ranges(
		&self,
		pat: &Regex,
		src: &str,
		is_global: bool,
	) -> Vec<(Span, String)> {
		let matches = pat.find_iter(src);
		let mut ret = Vec::new();
		for m in matches {
			ret.push((
				Span::new(m.start() as u32, m.end() as u32),
				self.text(src, &m),
			));
			if !is_global {
				break;
			}
		}
		debug_assert!(
			!ret.is_empty(),
			"we only get here after the match was validated"
		);
		ret
	}
}

fn report_syntax_error(
	alloc: &Allocator,
	e: OxcDiagnostic,
	src: &str,
	pat: &Regex,
	rewrite: &Rewrite<'_>,
	is_global: bool,
	report: SyntaxErrorReport<'_>,
) -> WrappedOxcDiagnostic {
	let SyntaxErrorReport::Formatted { file_name } = report else {
		return WrappedOxcDiagnostic::from(e);
	};
	let ranges = rewrite.ranges(pat, src, is_global);
	match format_syntax_error(alloc, e.clone(), src, &ranges, file_name) {
		Ok(formatted) => formatted,
		Err(err) => {
			warn!("Failed to format syntax error, reporting it raw: {err:?}");
			WrappedOxcDiagnostic::from(e)
		}
	}
}

fn get_repl_regex<'r>(
	replacement: &'r Replacement,
	events: &mut Vec<ApplyEvent>,
) -> Option<&'r Regex> {
	match &replacement.match_.v {
		// compile_patch_regexes rewrites every string match into a regex
		Match::Str(_) => unreachable!(),
		Match::Regex(v) => match v.regex() {
			Ok(r) => Some(r),
			Err(e) => {
				events.push(ApplyEvent::BadRegex {
					regex_span: replacement.match_.s.into(),
					source: e.clone(),
					expanded: format!("/{}/{}", v.pattern, v.flags),
				});
				None
			}
		},
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchTarget {
	Found,
	NotFound,
}

/// Whether the replacement should run, reporting why not when it should not.
fn check_match_target(
	pat: &Regex,
	src: &str,
	is_global: bool,
	no_warn: bool,
	replacement: &Replacement,
	events: &mut Vec<ApplyEvent>,
) -> MatchTarget {
	let mut it = pat.find_iter(src);

	if it.next().is_none() {
		events.push(
			ApplyEvent::MatchNotFound {
				match_span: replacement.match_.s.into(),
			}
			.suppress_if(no_warn),
		);
		return MatchTarget::NotFound;
	}

	if !is_global && it.next().is_some() {
		events.push(ApplyEvent::MatchAmbiguous {
			match_span: replacement.match_.s.into(),
		});
	}

	MatchTarget::Found
}
