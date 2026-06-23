//! "Go to definition" handler. Covers both:
//! * Direct `wreq(N)` numeric-id navigation — cursor on `42` in
//!   `wreq(42)` jumps to `.modules/42.js`.
//! * Cross-module access chains — cursor on `r.A` where `r = wreq(N)`
//!   jumps to where `A` is defined inside module `N`, following any
//!   re-exports along the way.
//!
//! Reuses the shared `CrossModuleData` cached on `SessionState` (built
//! once per session, invalidated on download / purge) so the disk scan
//! isn't repeated per request.

use std::{mem, path::PathBuf, sync::Arc};

use oxc::{allocator::Allocator, span::Span};
use tower_lsp::{
	jsonrpc::Result as LspResult,
	lsp_types::{
		GotoDefinitionParams,
		GotoDefinitionResponse,
		Location,
		Position,
		Range,
		Url,
	},
};
use vencord_ast_parser::VencordAstParser;
use webpack_ast_parser::{WebpackAstParser, bundle};

use crate::{
	lsp::{
		self,
		Backend,
		cross_module::{CrossModuleCtx, CrossModuleData},
		get_doc,
		hl,
		references::get_or_build_data,
	},
	state::Document,
};

pub async fn goto_definition(
	backend: &Backend,
	params: GotoDefinitionParams,
) -> LspResult<Option<GotoDefinitionResponse>> {
	let uri = params
		.text_document_position_params
		.text_document
		.uri;
	let pos = params
		.text_document_position_params
		.position;
	let Some(doc) = get_doc(&backend.state, &uri) else {
		return Ok(None);
	};

	// Vencord plugin files: cursor on a capture-group use in a replacement
	// (e.g. `$1`) jumps to that group's definition in the regex. Handled
	// before the webpack path since the two operate on disjoint file kinds.
	if let Some(loc) = capture_group_definition(backend, doc.clone(), pos).await
	{
		return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
	}

	// Vencord plugin files: cursor on a `$self.<prop>` reference in a
	// replacement jumps to where `<prop>` is defined in the `definePlugin`
	// object. Disjoint from the capture-group path above.
	if let Some(loc) = self_prop_definition(doc.clone(), pos).await {
		return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
	}

	// `WebpackAstParser` expects extracted webpack module source. Skip
	// vencord plugin files / non-JS, and bail before we hand the source
	// to oxc on any JS file that isn't an extracted webpack module —
	// parsing arbitrary editor buffers here would be wasted work.
	if doc.language_id != "javascript"
		|| !WebpackAstParser::is_webpack_module(&doc.text)
	{
		return Ok(None);
	}

	let Some(cache_root) = backend
		.state
		.module_cache
		.read()
		.await
		.root()
		.map(PathBuf::from)
	else {
		return Ok(None);
	};

	let Some(data) = get_or_build_data(&backend.state, cache_root).await else {
		return Ok(None);
	};

	let text = doc.text.clone();
	let locs = tokio::task::spawn_blocking(move || {
		resolve_definitions(data, &text, pos)
	})
	.await
	.ok()
	.flatten();

	Ok(locs.map(GotoDefinitionResponse::Array))
}

/// Resolves a capture-group use under the cursor to its definition span in the
/// regex. Operates on the cached vencord patch parse (reused across requests
/// when the document version is unchanged), so it stays cheap to attempt on
/// every `goto_definition` before falling through to the webpack path.
async fn capture_group_definition(
	backend: &Backend,
	doc: Document,
	pos: Position,
) -> Option<Location> {
	let state = Arc::clone(&backend.state);
	tokio::task::spawn_blocking(move || {
		let offset = ast_parser::get_offset_from_line_and_column(
			&doc.text,
			pos.line,
			pos.character,
		);
		let patches = lsp::get_patches(&state, &doc)?;
		let span = hl::capture_definition_span(&patches, offset)?;
		Some(Location {
			uri: doc.uri.clone(),
			range: span_to_range(&doc.text, span),
		})
	})
	.await
	.ok()
	.flatten()
}

/// Resolves a `$self.<prop>` reference under the cursor to where `<prop>` is
/// defined in the plugin's `definePlugin({...})` object. Parses the plugin
/// fresh (cheap, on-demand) rather than reusing the patch cache, since the key
/// definition spans come from the plugin info, not the patches.
async fn self_prop_definition(
	doc: Document,
	pos: Position,
) -> Option<Location> {
	tokio::task::spawn_blocking(move || {
		let offset = ast_parser::get_offset_from_line_and_column(
			&doc.text,
			pos.line,
			pos.character,
		);
		let alloc = Allocator::new();
		let parser = VencordAstParser::try_new(&alloc, &doc.text, None).ok()?;
		let span = parser.self_reference_definition(offset)?;
		Some(Location {
			uri: doc.uri.clone(),
			range: span_to_range(&doc.text, span),
		})
	})
	.await
	.expect("Failed to join task")
}

fn resolve_definitions(
	data: Arc<CrossModuleData>,
	source: &str,
	pos: Position,
) -> Option<Vec<Location>> {
	let ctx = CrossModuleCtx::from_data(data);

	let alloc = ctx.alloc();
	// SAFETY: `source` outlives this function. The parser borrows from
	// it but never escapes `resolve_definitions` — we destructure into
	// owned `Location`s before returning.
	let source_static: &'static str =
		unsafe { mem::transmute::<&str, &'static str>(source) };
	let mut parser = WebpackAstParser::try_new(alloc, source_static).ok()?;
	parser.set_module_cache(ctx.cache_ref());
	parser.set_module_dep_provider(ctx.dep_provider_ref());

	let offset = ast_parser::get_offset_from_line_and_column(
		source,
		pos.line,
		pos.character,
	);
	let defs = match parser.generate_definitions(offset) {
		Ok(d) => d,
		Err(e) => {
			tracing::debug!(?e, "generate_definitions failed");
			return None;
		}
	};
	if defs.is_empty() {
		return None;
	}

	let mut out = Vec::with_capacity(defs.len());
	for def in &defs {
		if let Some(loc) = definition_to_lsp_location(&ctx, source, def) {
			out.push(loc);
		}
	}
	(!out.is_empty()).then_some(out)
}

fn definition_to_lsp_location(
	ctx: &CrossModuleCtx,
	open_source: &str,
	def: &bundle::Definition<'_>,
) -> Option<Location> {
	let uri = match &def.location {
		bundle::Location::Path(s) => Url::parse(s).ok()?,
		// Inline locations point at a parser source we don't have a URI
		// for directly — recover one via the module id.
		bundle::Location::Inline(_) => ctx.module_file_uri(def.module_id)?,
	};
	// Range conversion needs the source the span lives in. Direct-module
	// definitions ship `Span::default()` (start = end = 0), which maps
	// cleanly to a zero range at the file head regardless of what source
	// we look it up against.
	let src = ctx
		.module_source(def.module_id)
		.unwrap_or(open_source);
	Some(Location {
		uri,
		range: span_to_range(src, def.range),
	})
}

fn span_to_range(source: &str, span: Span) -> Range {
	let ((sl, sc), (el, ec)) = ast_parser::span_line_and_column(source, span);
	Range {
		start: Position {
			line: sl,
			character: sc,
		},
		end: Position {
			line: el,
			character: ec,
		},
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::*;
	use tempfile::TempDir;
	use tower_lsp::lsp_types::Url;

	#[test]
	fn direct_module_id_resolves_to_cached_path() {
		// `wreq(42)` — cursor on `42` should resolve to `.modules/42.js`.
		const SRC: &str = "0,function(e,t,n){var x=n(42)}";
		let tmp = TempDir::new().unwrap();
		// The cross-module data scan only loads files whose stems parse
		// as a u32 — keep `42.js` simple but parseable enough.
		fs::write(
			tmp.path().join("42.js"),
			"// Webpack Module 42\n0,function(e,t,n){}\n",
		)
		.unwrap();

		let data =
			Arc::new(CrossModuleData::build(tmp.path().to_owned()).unwrap());
		let pos = position_of(SRC, "42");
		let locs = resolve_definitions(data, SRC, pos)
			.expect("expected at least one definition");
		assert_eq!(locs.len(), 1);
		let expected_uri =
			Url::from_file_path(tmp.path().join("42.js")).unwrap();
		assert_eq!(locs[0].uri, expected_uri);
	}

	#[test]
	fn returns_none_when_cursor_not_on_module_id() {
		const SRC: &str = "0,function(e,t,n){var x=n(42)}";
		let tmp = TempDir::new().unwrap();
		fs::write(
			tmp.path().join("42.js"),
			"// Webpack Module 42\n0,function(e,t,n){}\n",
		)
		.unwrap();
		let data =
			Arc::new(CrossModuleData::build(tmp.path().to_owned()).unwrap());
		// Cursor on the `var` keyword — not a member-access target.
		let pos = position_of(SRC, "var");
		assert!(resolve_definitions(data, SRC, pos).is_none());
	}

	fn position_of(src: &str, needle: &str) -> Position {
		let offset = src
			.find(needle)
			.expect("needle not in source") as u32
			+ 1; // land inside the token, not on its first byte
		let (line, character) = ast_parser::get_line_and_column(src, offset);
		Position { line, character }
	}
}
