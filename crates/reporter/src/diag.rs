use derive_more::IsVariant;
use explorer_types::ModuleId;
use miette::{Diagnostic, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::reporter::WrappedOxcDiagnostic;

mod serde_regress_error;

#[derive(
	Error,
	Debug,
	Diagnostic,
	IsVariant,
	Serialize,
	Deserialize,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Clone,
)]
pub enum ReporterError {
	#[error("Bad Regex Syntax")]
	#[diagnostic[
        code(reporter::bad_regex_syntax),
        severity(Error),
        help("The regex was expanded to {expanded}"),
    ]]
	BadRegexSyntax {
		plugin_id: u16,
		#[source]
		#[serde(with = "serde_regress_error")]
		source: regress::Error,
		#[label("From this regex")]
		regex_span: SourceSpan,
		expanded: String,
	},
	#[error("Replace Match Not Found")]
	#[diagnostic[
        code(reporter::replace::match_not_found),
        severity(Error),
        help("This error occurred in module {module_id}")
    ]]
	ReplaceMatchNotFound {
		plugin_id: u16,
		#[label("Caused by this match")]
		match_span: SourceSpan,
		module_id: ModuleId,
	},
	#[error("Replace Match Ambiguous")]
	#[diagnostic[
        code(reporter::replace::match_ambiguous),
        severity(Warning),
        help("This error occurred in module {module_id}")        
    ]]
	ReplaceMatchAmbiguous {
		plugin_id: u16,
		#[label("Caused by this match")]
		match_span: SourceSpan,
		module_id: ModuleId,
	},
	#[error("Replace Syntax Error")]
	#[diagnostic[
        code(reporter::replace::syntax_error),
        severity(Error),
        help("This error occurred in module {module_id}"),
    ]]
	ReplaceSyntaxError {
		plugin_id: u16,
		#[label("Caused by this replacement")]
		replace_span: SourceSpan,
		#[source]
		#[diagnostic_source]
		cause: WrappedOxcDiagnostic,
		module_id: ModuleId,
	},
	#[error("Find Ambiguous")]
	#[diagnostic[
        code(reporter::find::ambiguous),
        severity(Error),
        help("Modules {ok_ids:?} matched and applied without issue.\nModules {err_ids:?} matches, but errored while applying"),
    ]]
	// TODO: Add related failures here something like Option<Vec<ReporterError>>
	FindAmbiguous {
		plugin_id: u16,
		#[label(
			"This find matches more than one module. Make it more specific!"
		)]
		find_span: SourceSpan,
		ok_ids: Vec<u32>,
		err_ids: Vec<u32>,
	},
	#[error("Find Too Broad")]
	#[diagnostic[
        code(reporter::find::broad),
        severity(Warning),
        help("This patch executed without issue on module {ok_id}; however, it matched and failed to execute on modules {err_ids:?}.{extra_help}"),
    ]]
	FindAmbiguousRecoverable {
		plugin_id: u16,
		#[label(
			"This find matches more than one module. Make it more specific!"
		)]
		find_span: SourceSpan,
		ok_id: ModuleId,
		err_ids: Vec<u32>,
		extra_help: &'static str,
	},
	#[error("No matches found")]
	#[diagnostic[
        code(reporter::find::not_found),
        severity(Error),
		help("for help with fixing this specific patch, you can try `reporter fix {patch_hash:x}`")
    ]]
	FindNotFound {
		plugin_id: u16,
		#[label("This find failed to match anything")]
		find_span: SourceSpan,
		patch_hash: u64,
	},
	#[error(transparent)]
	NoWarn(Box<Self>),
}
impl ReporterError {
	pub const fn plugin_id(&self) -> u16 {
		match self {
			Self::BadRegexSyntax { plugin_id, .. }
			| Self::ReplaceMatchNotFound { plugin_id, .. }
			| Self::ReplaceMatchAmbiguous { plugin_id, .. }
			| Self::ReplaceSyntaxError { plugin_id, .. }
			| Self::FindNotFound { plugin_id, .. }
			| Self::FindAmbiguous { plugin_id, .. }
			| Self::FindAmbiguousRecoverable { plugin_id, .. } => *plugin_id,
			Self::NoWarn(e) => e.plugin_id(),
		}
	}

	pub fn cause_span(&self) -> SourceSpan {
		match self {
			Self::ReplaceMatchNotFound {
				match_span: span, ..
			}
			| Self::ReplaceMatchAmbiguous {
				match_span: span, ..
			}
			| Self::ReplaceSyntaxError {
				replace_span: span, ..
			}
			| Self::FindAmbiguous {
				find_span: span, ..
			}
			| Self::FindAmbiguousRecoverable {
				find_span: span, ..
			}
			| Self::FindNotFound {
				find_span: span, ..
			}
			| Self::BadRegexSyntax {
				regex_span: span, ..
			} => *span,
			Self::NoWarn(i) => i.cause_span(),
		}
	}

	pub const fn module_id(&self) -> Option<ModuleId> {
		match self {
			Self::FindNotFound { .. }
			| Self::BadRegexSyntax { .. }
			| Self::FindAmbiguous { .. } => None,
			Self::ReplaceSyntaxError { module_id, .. }
			| Self::ReplaceMatchAmbiguous { module_id, .. }
			| Self::FindAmbiguousRecoverable {
				ok_id: module_id, ..
			}
			| Self::ReplaceMatchNotFound { module_id, .. } => Some(*module_id),
			Self::NoWarn(e) => e.module_id(),
		}
	}

	pub fn sort_inner_data(&mut self) {
		match self {
			Self::FindAmbiguous {
				ok_ids, err_ids, ..
			} => {
				ok_ids.sort_unstable();
				err_ids.sort_unstable();
			}
			Self::FindAmbiguousRecoverable { err_ids, .. } => {
				err_ids.sort_unstable();
			}
			Self::NoWarn(reporter_error) => {
				reporter_error.sort_inner_data();
			}
			Self::BadRegexSyntax { .. }
			| Self::ReplaceMatchNotFound { .. }
			| Self::ReplaceMatchAmbiguous { .. }
			| Self::ReplaceSyntaxError { .. }
			| Self::FindNotFound { .. } => {}
		}
	}
}
