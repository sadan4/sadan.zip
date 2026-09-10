use tower_lsp::lsp_types::{GotoDefinitionParams, GotoDefinitionResponse};
use tracing::{debug, info, warn};
use webpack_ast_parser::WebpackAstParser;

use crate::{LspResult, lsp};

impl lsp::Server {
	pub(crate) async fn provide_definition(&self, params: GotoDefinitionParams) -> LspResult<Option<GotoDefinitionResponse>> {
		let uri = &params.text_document_position_params.text_document.uri;
		let Some(doc) = self.files.get(uri) else {
			warn!("no document found for uri");
			return Ok(None)
		};
		if !WebpackAstParser::is_webpack_module(&doc.text) {
			debug!("document is not a webpack module, skipping definition");
			return Ok(None);
		}
		info!("TODO: provide definition");
		Ok(None)
	}
}
