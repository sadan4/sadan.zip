use std::{cmp::Ordering, sync::Arc};

use ast_parser::get_offset_from_line_and_column;
use dashmap::DashMap;
use smol_str::SmolStr;
use tower_lsp::lsp_types::{
	DidChangeTextDocumentParams,
	DidCloseTextDocumentParams,
	DidOpenTextDocumentParams,
	TextDocumentContentChangeEvent,
	TextDocumentItem,
	Url,
	notification::DidCloseTextDocument,
};
use tracing::{debug, warn};

#[derive(Clone, Default)]
pub struct Files(Arc<Inner>);

#[derive(Default)]
struct Inner {
	cache: DashMap<Url, TextDocumentItem>,
}

fn update_document(
	old: &mut String,
	changes: &[TextDocumentContentChangeEvent],
) {
	for change in changes {
		if let Some(range) = &change.range {
			let start = get_offset_from_line_and_column(
				old,
				range.start.line,
				range.start.character,
			) as usize;
			let end = get_offset_from_line_and_column(
				old,
				range.end.line,
				range.end.character,
			) as usize;
			if old.is_char_boundary(start) || old.is_char_boundary(end) {
				warn!(
					start = start,
					end = end,
					len = old.len(),
					"received change with invalid range, skipping"
				);
				continue;
			}
			old.replace_range(start..end, &change.text);
		} else {
			debug!("recieved change without range, replacing entire document");
			old.clone_from(&change.text);
		}
	}
}

impl Files {
	pub fn get(&self, uri: &Url) -> Option<TextDocumentItem> {
		self.0.cache.get(uri).map(|e| e.clone())
	}

	pub fn handle_open(&self, params: DidOpenTextDocumentParams) {
		if self
			.0
			.cache
			.contains_key(&params.text_document.uri)
		{
			warn!("document already open, overwriting");
		}
		self.0
			.cache
			.insert(params.text_document.uri.clone(), params.text_document);
	}

	pub fn handle_change(&self, params: DidChangeTextDocumentParams) {
		let new = params.text_document;
		let changes = params.content_changes;
		if let Some(mut doc) = self.0.cache.get_mut(&new.uri) {
			if doc.version >= new.version {
				warn!(
					old_version = doc.version,
					new_version = new.version,
					"received change with older version than current document, skipping"
				);
				return;
			}
			update_document(&mut doc.text, &changes);
		} else {
			warn!("received change for document that is not open, skipping");
		}
	}

	pub fn handle_close(&self, params: DidCloseTextDocumentParams) {
		let uri = params.text_document.uri;
		if self.0.cache.remove(&uri).is_none() {
			warn!("received close for document that is not open, skipping");
		}
	}
}
