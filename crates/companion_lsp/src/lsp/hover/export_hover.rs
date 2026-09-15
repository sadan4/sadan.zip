use anyhow::{Context as _, Result};
use explorer_types::SpannedId;
use parser_diag::LocalSource;
use tower_lsp_server::ls_types::{HoverParams, Range};
use tracing::error;
use webpack_ast_parser::WebpackAstParser;

use crate::lsp;

impl lsp::Server {
	pub(super) async fn provide_export_hover(
		&self,
		params: &HoverParams,
	) -> Result<Option<(String, Range)>> {
		let uri = &params
			.text_document_position_params
			.text_document
			.uri;
		let Some(doc) = self.files.get(uri) else {
			return Ok(None);
		};
		let Some(SpannedId { id, span: _ }) =
			WebpackAstParser::parse_module_id(&doc.text)
		else {
			return Ok(None);
		};
		let parser = self
			.module_cache
			.get_for_uri(uri)
			.get_parser(id)
			.await
			.context("Failed to get parser for module")?;
		let pos = lsp::cursor_offset(
			&doc,
			&parser,
			params
				.text_document_position_params
				.position,
		)?;
		match parser
			.parser()
			.generate_hover(pos)
			.await
		{
			Ok(Some((span, content))) => {
				Ok(Some((content.into(), doc.range_for_span(span))))
			}
			Ok(None) => Ok(None),
			Err(e) => {
				let e = LocalSource {
					name: uri.as_str(),
					source: parser.get_source(),
					inner: miette::Report::from(e),
				};
				error!("Failed to generate hover: {e:?}");
				Ok(None)
			}
		}
	}
}
