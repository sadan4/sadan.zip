use tower_lsp_server::ls_types::{CodeLens, CodeLensParams};
use tracing::{debug, instrument, trace};

use crate::{LspResult, lsp, util::uri};

mod patch;
mod plugin_def;
mod webpack;

pub use patch::{PatchLensArgs, is_plugin_path};

impl lsp::Server {
	#[instrument(skip_all, fields(uri =% params.text_document.uri.as_str()))]
	pub(super) async fn provide_lenses(
		&self,
		params: CodeLensParams,
	) -> LspResult<Option<Vec<CodeLens>>> {
		let uri = &params.text_document.uri;
		let path = match uri::to_path(uri) {
			Ok(p) => p,
			Err(e) => {
				trace!("uri is not a file path, skipping lenses: {e}");
				return Ok(None);
			}
		};
		let Some(doc) = self.files.get(uri) else {
			debug!("no document found for uri, skipping lenses");
			return Ok(None);
		};
		Ok(Some(patch::patch_lenses(uri, &doc, &path)))
	}
}
