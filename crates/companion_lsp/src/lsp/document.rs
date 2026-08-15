use deno_tower_lsp::lsp_types::{
	DidChangeTextDocumentParams,
	DidCloseTextDocumentParams,
	DidOpenTextDocumentParams,
	Range,
};

use crate::{
	lsp::{Backend, diagnostics, patch_helper},
	state::Document,
};

pub fn on_did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
	let td = params.text_document;
	let doc = Document {
		uri: td.uri.clone(),
		version: td.version,
		language_id: td.language_id,
		text: td.text,
	};
	backend
		.state
		.documents
		.insert(td.uri.clone(), doc);
	diagnostics::schedule(
		backend.state.clone(),
		backend.client.clone(),
		td.uri,
	);
}

pub fn on_did_change(backend: &Backend, params: DidChangeTextDocumentParams) {
	let uri = params.text_document.uri.clone();
	let version = params.text_document.version;
	let Some(mut entry) = backend.state.documents.get_mut(&uri) else {
		tracing::warn!(?uri, "did_change for unknown document");
		return;
	};

	for change in params.content_changes {
		match change.range {
			Some(range) => {
				apply_range_change(&mut entry.text, range, &change.text);
			}
			None => entry.text = change.text,
		}
	}
	entry.version = version;
	drop(entry);

	diagnostics::schedule(
		backend.state.clone(),
		backend.client.clone(),
		uri.clone(),
	);

	// If a Patch Helper is tracking this source, re-resolve and push fresh
	// content. Fire-and-forget — patch helper updates are best-effort and
	// shouldn't block the document-sync path.
	let client = backend.client.clone();
	let state = backend.state.clone();
	tokio::spawn(async move {
		patch_helper::on_source_change(client, state, uri).await;
	});
}

pub fn on_did_close(backend: &Backend, params: DidCloseTextDocumentParams) {
	let uri = params.text_document.uri;
	backend.state.documents.remove(&uri);
	backend.state.patch_cache.remove(&uri);

	// Fire-and-forget so the LSP sync path isn't blocked by the client
	// notification round-trip.
	let client = backend.client.clone();
	let state = backend.state.clone();
	tokio::spawn(async move {
		patch_helper::on_source_close(&client, &state, &uri).await;
	});
}

fn apply_range_change(text: &mut String, range: Range, replacement: &str) {
	let start = ast_parser::get_offset_from_line_and_column(
		text,
		range.start.line,
		range.start.character,
	) as usize;
	let end = ast_parser::get_offset_from_line_and_column(
		text,
		range.end.line,
		range.end.character,
	) as usize;
	let start = start.min(text.len());
	let end = end.min(text.len()).max(start);
	text.replace_range(start..end, replacement);
}
