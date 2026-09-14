use explorer_types::SpannedId;
use parser_diag::LocalSource;
use tower_lsp::lsp_types::{Location, ReferenceParams};
use tracing::{debug, warn};
use webpack_ast_parser::{WebpackAstParser, bundle};

use crate::{LspResult, lsp};

impl lsp::Server {
	pub async fn gen_references(
		&self,
		params: ReferenceParams,
	) -> LspResult<Option<Vec<Location>>> {
		let uri = &params
			.text_document_position
			.text_document
			.uri;
		let Some(doc) = self.files.get(uri) else {
			warn!("no document found for uri");
			return Ok(None);
		};
		if !WebpackAstParser::is_webpack_module(&doc.text) {
			debug!("document is not a webpack module, skipping references");
			return Ok(None);
		}
		let Some(SpannedId {
			id: module_id,
			span: _,
		}) = WebpackAstParser::parse_module_id(&doc.text)
		else {
			debug!(
				"failed to parse module id from document, skipping references"
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
			params.text_document_position.position,
		)?;
		let refs = match p.generate_references(offset).await {
			Ok(refs) => refs,
			Err(e) => {
				let err = LocalSource {
					inner: miette::Report::from(e),
					source: p.get_source(),
					name: uri.as_str(),
				};
				warn!("Failed to generate references:{err:?}");
				return Ok(None);
			}
		};
		let mut ret = Vec::with_capacity(refs.len());
		for reference in refs {
			let ref_parser = match module_cache
				.get_parser(reference.module_id)
				.await
			{
				Ok(p) => p,
				Err(e) => {
					warn!(
						module_id = %reference.module_id,
						"Failed to get parser for module: {e:?}"
					);
					continue;
				}
			};
			let ref_src = ref_parser.get_source();
			let range = self
				.files
				.encoding()
				.range_in(ref_src, reference.range);
			let bundle::Location::Path(uri) = reference.location else {
				warn!("TODO: handle reference locations other than Paths");
				continue;
			};
			ret.push(Location { uri, range });
		}
		Ok(Some(ret))
	}
}
