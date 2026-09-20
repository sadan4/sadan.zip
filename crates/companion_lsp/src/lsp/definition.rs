use explorer_types::SpannedId;
use parser_diag::LocalSource;
use tower_lsp_server::ls_types::{
	GotoDefinitionParams,
	GotoDefinitionResponse,
	Location,
};
use tracing::{debug, error, warn};
use webpack_ast_parser::{WebpackAstParser, bundle};

use crate::{LspResult, lsp, util::uri};

impl lsp::Server {
	pub(crate) async fn provide_definition(
		&self,
		params: GotoDefinitionParams,
	) -> LspResult<Option<GotoDefinitionResponse>> {
		let uri = &params
			.text_document_position_params
			.text_document
			.uri;
		let Some(doc) = self.files.get(uri) else {
			warn!("no document found for uri");
			return Ok(None);
		};
		if !WebpackAstParser::is_webpack_module(&doc.text) {
			debug!("document is not a webpack module, skipping definition");
			return Ok(None);
		}
		let Some(SpannedId {
			id: module_id,
			span: _,
		}) = WebpackAstParser::parse_module_id(&doc.text)
		else {
			debug!(
				"failed to parse module id from document, skipping definition"
			);
			return Ok(None);
		};
		let module_cache = self.module_cache.get_for_uri(uri);
		let module = match module_cache.get_parser(module_id).await {
			Ok(module) => module,
			Err(e) => {
				warn!(%module_id, "Failed to find module parser: {e:?}");
				return Ok(None);
			}
		};
		let p = module.parser();
		let offset = lsp::cursor_offset(
			&doc,
			&module,
			params
				.text_document_position_params
				.position,
		)?;
		let defs = match p.generate_definitions(offset).await {
			Ok(defs) => defs,
			Err(e) => {
				let err = LocalSource {
					inner: miette::Report::from(e),
					source: p.get_source(),
					name: uri.as_str(),
				};
				warn!("Failed to generate definitions:{err:?}");
				return Ok(None);
			}
		};
		let mut ret = Vec::with_capacity(defs.len());
		for def in defs {
			let def_parser = match module_cache
				.get_parser(def.module_id)
				.await
			{
				Ok(p) => p,
				Err(e) => {
					error!(
						"Failed to get parser for module {module_id:?}: {e:?}"
					);
					return Ok(None);
				}
			};
			let def_src = def_parser.get_source();
			let range = self
				.files
				.encoding()
				.range_in(def_src, def.range);
			let bundle::Location::Path(uri) = def.location else {
				warn!("TODO: handle definition locations other than Paths");
				return Ok(None);
			};
			ret.push(Location {
				uri: uri::from_url(&uri),
				range,
			});
		}
		Ok(Some(GotoDefinitionResponse::Array(ret)))
	}
}
