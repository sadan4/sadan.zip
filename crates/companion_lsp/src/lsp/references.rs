//! "Find all references" handler. Walks every cached webpack module that
//! imports the pivot module, using the shared `CrossModuleData` cached on
//! `SessionState` (built once per session, invalidated on download /
//! purge) to avoid re-scanning `.modules/` on every request.

use std::{mem, path::PathBuf, sync::Arc};

use oxc::span::Span;
use tower_lsp::{
	jsonrpc::Result as LspResult,
	lsp_types::{Location, Position, Range, ReferenceParams, Url},
};
use webpack_ast_parser::{WebpackAstParser, bundle};

use crate::{
	lsp::{
		Backend,
		cross_module::{CrossModuleCtx, CrossModuleData},
		get_doc,
	},
	state::SharedState,
};

pub async fn references(
	backend: &Backend,
	params: ReferenceParams,
) -> LspResult<Option<Vec<Location>>> {
	let uri = params
		.text_document_position
		.text_document
		.uri
		.clone();
	let pos = params.text_document_position.position;
	let Some(doc) = get_doc(&backend.state, &uri) else {
		return Ok(None);
	};
	// Only extracted webpack modules carry the headers the parser keys
	// off of; skip everything else before we pay for the cross-module
	// ctx build (which scans every cached module).
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
		compute_references(data, &text, pos)
	})
	.await
	.ok()
	.flatten();
	Ok(locs)
}

/// Returns the cached `CrossModuleData` for `cache_root`, building (and
/// caching) it lazily if missing or if the workspace root has shifted.
/// Holds the write lock across the blocking scan so concurrent callers
/// can't race into duplicate builds.
pub(super) async fn get_or_build_data(
	state: &SharedState,
	cache_root: PathBuf,
) -> Option<Arc<CrossModuleData>> {
	// Fast path: read lock, hit the cache.
	{
		let guard = state.cross_module_data.read().await;
		if let Some(d) = guard.as_ref()
			&& d.root() == cache_root
		{
			return Some(d.clone());
		}
	}
	// Slow path: take the write lock, double-check, then build.
	let mut guard = state.cross_module_data.write().await;
	if let Some(d) = guard.as_ref()
		&& d.root() == cache_root
	{
		return Some(d.clone());
	}
	let build_root = cache_root.clone();
	let built =
		tokio::task::spawn_blocking(move || CrossModuleData::build(build_root))
			.await
			.ok()?;
	let data = match built {
		Ok(d) => Arc::new(d),
		Err(e) => {
			tracing::debug!(?e, "cross-module data build failed");
			return None;
		}
	};
	*guard = Some(data.clone());
	Some(data)
}

fn compute_references(
	data: Arc<CrossModuleData>,
	source: &str,
	pos: Position,
) -> Option<Vec<Location>> {
	let ctx = CrossModuleCtx::from_data(data);

	let alloc = ctx.alloc();
	// SAFETY: `source` outlives this function. The parser borrows from
	// it, but never escapes `compute_references` — we destructure into
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
	let refs = match parser.generate_references(offset) {
		Ok(r) => r,
		Err(e) => {
			tracing::debug!(?e, "generate_references failed");
			return None;
		}
	};
	if refs.is_empty() {
		return None;
	}

	let mut out = Vec::with_capacity(refs.len());
	for r in &refs {
		if let Some(loc) = reference_to_lsp(&ctx, source, r) {
			out.push(loc);
		}
	}
	(!out.is_empty()).then_some(out)
}

fn reference_to_lsp(
	ctx: &CrossModuleCtx,
	open_source: &str,
	r: &bundle::Reference<'_>,
) -> Option<Location> {
	let uri = match &r.location {
		bundle::Location::Path(s) => Url::parse(s).ok()?,
		// Inline locations point at a parser source we don't have a URI
		// for directly — recover one via the module id.
		bundle::Location::Inline(_) => ctx.module_file_uri(r.module_id)?,
	};
	// For line/column we need the source the span is in. Prefer the
	// ctx's loaded copy (which the parser actually walked); fall back to
	// the open buffer if the reference came back from the pivot module
	// before it landed in the cache.
	let src = ctx
		.module_source(r.module_id)
		.unwrap_or(open_source);
	Some(Location {
		uri,
		range: span_to_range(src, r.range),
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
