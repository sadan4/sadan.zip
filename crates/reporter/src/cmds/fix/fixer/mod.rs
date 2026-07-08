mod find_not_found;
use std::sync::Arc;

use explorer_server_core::Channel;
use miette_ctx::ErrCtx;

use crate::{
	cmds::fix::find_last_build::BuildDiff,
	diag::ReporterError,
	vc::Plugin,
};

pub struct Todo;
pub async fn dispatch(
	diff: BuildDiff,
	patch: Arc<Vec<Plugin>>,
	diag: ReporterError,
	channel: Channel,
) -> miette::Result<Todo> {
	match diag {
		ReporterError::BadRegexSyntax { .. }
		| ReporterError::ReplaceMatchNotFound { .. }
		| ReporterError::ReplaceMatchAmbiguous { .. }
		| ReporterError::ReplaceSyntaxError { .. }
		| ReporterError::FindAmbiguous { .. }
		| ReporterError::FindAmbiguousRecoverable { .. }
		| ReporterError::NoWarn(..) => todo!("handle {diag:#?}"),
		ReporterError::FindNotFound { .. } => {
			tokio::task::spawn_blocking(move || {
				find_not_found::Fixer::new(diff, patch, diag, channel).fix()
			})
			.await
			.context("Join Error")?
		}
	}
}
