use derive_more::IsVariant;
use explorer_types::ModuleId;
use miette::{Diagnostic, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::reporter::WrappedOxcDiagnostic;

mod serde_regress_error;

#[derive(Error, Debug, Diagnostic, IsVariant, Serialize, Deserialize)]
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
		#[label("Caused by this match")]
		match_span: SourceSpan,
		module_id: ModuleId,
		plugin_id: u16,
	},
	#[error("Replace Match Ambiguous")]
	#[diagnostic[
        code(reporter::replace::match_ambiguous),
        severity(Warning),
        help("This error occurred in module {module_id}")        
    ]]
	ReplaceMatchAmbiguous {
		#[label("Caused by this match")]
		match_span: SourceSpan,
		plugin_id: u16,
		module_id: ModuleId,
	},
	#[error("Replace Syntax Error")]
	#[diagnostic[
        code(reporter::replace::syntax_error),
        severity(Error),
        help("This error occurred in module {module_id}"),
    ]]
	ReplaceSyntaxError {
		#[label("Caused by this replacement")]
		replace_span: SourceSpan,
		#[source]
		#[diagnostic_source]
		cause: WrappedOxcDiagnostic,
		module_id: ModuleId,
		plugin_id: u16,
	},
	#[error("Find Ambiguous")]
	#[diagnostic[
        code(reporter::find::ambiguous),
        severity(Error),
        help("Modules {ok_ids:?} matched and applied without issue.\nModules {err_ids:?} matches, but errored while applying"),
    ]]
	// TODO: Add related failures here something like Option<Vec<ReporterError>>
	FindAmbiguous {
		#[label(
			"This find matches more than one module. Make it more specific!"
		)]
		find_span: SourceSpan,
		plugin_id: u16,
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
		#[label(
			"This find matches more than one module. Make it more specific!"
		)]
		find_span: SourceSpan,
		plugin_id: u16,
		ok_id: ModuleId,
		err_ids: Vec<u32>,
		extra_help: &'static str,
	},
	#[error("No matches found")]
	#[diagnostic[
        code(reporter::find::not_found),
        severity(Error),
    ]]
	FindNotFound {
		#[label("This find failed to match anything")]
		find_span: SourceSpan,
		plugin_id: u16,
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
}
