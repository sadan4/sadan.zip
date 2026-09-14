use tower_lsp::lsp_types::{CodeLens, CodeLensParams};
use tracing::{debug, instrument};

use crate::{LspResult, lsp};

mod patch;
mod plugin_def;
mod webpack;

impl lsp::Server {
	#[instrument(skip_all, fields(uri =% params.text_document.uri))]
	pub(super) async fn provide_lenses(
		&self,
		params: CodeLensParams,
	) -> LspResult<Option<Vec<CodeLens>>> {
		let uri = &params.text_document.uri;
		let Ok(path) = uri.to_file_path() else {
			debug!("uri is not a file path, skipping lenses");
			return Ok(None);
		};
		let Some(doc) = self.files.get(uri) else {
			debug!("no document found for uri, skipping lenses");
			return Ok(None);
		};
		Ok(Some(patch::patch_lenses(uri, &doc, &path)))
	}
}
