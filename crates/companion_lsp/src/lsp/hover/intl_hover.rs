use anyhow::{Context as _, Result};
use explorer_types::SpannedId;
use smol_str::SmolStr;
use std::fmt::Write as _;
use tower_lsp_server::ls_types::{HoverParams, Range};
use tracing::warn;
use webpack_ast_parser::{WebpackAstParser, intl::resolve_unhashed_key};

use crate::{lsp, util::err::display_no_backtrace};

struct IntlToken {
	/// 6-char hashed key
	hashed: SmolStr,
	/// The resolved source key, if available
	source: Option<SmolStr>,
	/// TODO: pull this value from the bundle instead of querying
	/// the text value of the key
	resolved_value: Option<SmolStr>,
	span: Range,
}

impl IntlToken {
	fn render(&self) -> (String, Range) {
		let mut s = String::new();
		self.render_key(&mut s);
		self.render_copy_string(&mut s);
		self.render_resolved(&mut s);
		(s, self.span)
	}
	fn render_key(&self, to: &mut String) {
		let key = self
			.source
			.as_deref()
			.unwrap_or("no mapping found");
		write!(to, "{key}\n\n").unwrap();
	}
	fn render_copy_string(&self, to: &mut String) {
		let cmd_uri = if let Some(ref source) = self.source {
			lsp::Server::copy_string_cmd_uri(&format!("#{{intl::{source}}}"))
		} else {
			lsp::Server::copy_string_cmd_uri(&format!(
				"#{{intl::{hashed}::raw}}",
				hashed = self.hashed
			))
		};
		write!(to, "$(copy) [Copy as Find]({cmd_uri})\n\n").unwrap();
	}
	fn render_resolved(&self, to: &mut String) {
		let rv = self
			.resolved_value
			.as_deref()
			.unwrap_or("failed to fetch i18n value");
		write!(to, "{rv}\n\n").unwrap();
	}
}

impl lsp::Server {
	async fn mk_token(&self, span: Range, hashed: SmolStr) -> IntlToken {
		debug_assert_eq!(
			hashed.len(),
			6,
			"hashed key must be 6 characters long"
		);
		let source = resolve_unhashed_key(&hashed);
		let resolved_value = self
			.state
			.ws
			.lookup_intl_value(&hashed)
			.await
			.inspect_err(|e| {
				let e = display_no_backtrace(e);
				warn!("Failed to get value of intl string from client: {e}");
			})
			.ok();
		IntlToken {
			hashed,
			source,
			resolved_value,
			span,
		}
	}
	pub(super) async fn provide_intl_hover(
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
		let Some((span, key)) = parser.parser().get_i18n_key_at(pos) else {
			return Ok(None);
		};
		Ok(Some(
			self.mk_token(doc.range_for_span(span), key)
				.await
				.render(),
		))
	}
}
