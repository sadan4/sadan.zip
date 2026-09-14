use anyhow::{Context as _, Result};
use ast_parser::span_line_and_column;
use explorer_types::SpannedId;
use parser_diag::LocalSource;
use tower_lsp::lsp_types::{HoverParams, Position, Range};
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
			&doc.text,
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
				let ((start_line, start_col), (end_line, end_col)) =
					span_line_and_column(&doc.text, span);
				Ok(Some((
					content.into(),
					Range {
						start: Position {
							line: start_line,
							character: start_col,
						},
						end: Position {
							line: end_line,
							character: end_col,
						},
					},
				)))
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
