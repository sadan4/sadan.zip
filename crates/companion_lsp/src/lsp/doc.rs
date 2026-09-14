use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use memchr::{memchr_iter, memrchr};
use oxc::span::Span;
use tower_lsp::lsp_types::{
	DidChangeTextDocumentParams,
	DidCloseTextDocumentParams,
	DidOpenTextDocumentParams,
	GeneralClientCapabilities,
	Position,
	PositionEncodingKind,
	Range,
	TextDocumentContentChangeEvent,
	TextDocumentItem,
	Url,
};
use tracing::{debug, info, instrument, warn};

#[derive(Clone, Default)]
pub struct Files(Arc<Inner>);

#[derive(Default)]
struct Inner {
	cache: DashMap<Url, Arc<Document>>,
	/// The position encodings supported by the client. Client and server
	/// have to agree on the same position encoding to ensure that offsets
	/// (e.g. character position in a line) are interpreted the same on both
	/// sides.
	///
	/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#textDocuments>
	encoding: OnceLock<PositionEncoding>,
}

/// A type indicating how positions are encoded, specifically what column offsets mean.
///
/// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#positionEncodingKind>
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PositionEncoding {
	/// Character offsets count UTF-8 code units (i.e. bytes).
	Utf8,
	/// Character offsets count UTF-16 code units.
	///
	/// This is the default and must always be supported
	/// by servers.
	#[default]
	Utf16,
	/// Character offsets count UTF-32 code units.
	///
	/// Implementation note: these are the same as Unicode code points,
	/// so this `PositionEncodingKind` may also be used for an
	/// encoding-agnostic representation of character offsets.
	Utf32,
}

impl PositionEncoding {
	const fn kind(self) -> PositionEncodingKind {
		match self {
			Self::Utf8 => PositionEncodingKind::UTF8,
			Self::Utf16 => PositionEncodingKind::UTF16,
			Self::Utf32 => PositionEncodingKind::UTF32,
		}
	}

	/// The byte offset into `line` of a `character` column.
	///
	/// A column past the end of the line is clamped to the end of the line,
	/// and one that splits a character is clamped back to where that
	/// character starts.
	fn column_to_byte(self, line: &str, character: u32) -> usize {
		let character = character as usize;
		match self {
			Self::Utf8 => {
				let mut byte = character.min(line.len());
				// byte 0 is always a boundary, so this never underflows
				while !line.is_char_boundary(byte) {
					byte -= 1;
				}
				byte
			}
			Self::Utf16 => {
				let mut units = 0;
				for (byte, ch) in line.char_indices() {
					if units >= character {
						return byte;
					}
					units += ch.len_utf16();
					// the column split a surrogate pair
					if units > character {
						return byte;
					}
				}
				line.len()
			}
			Self::Utf32 => line
				.char_indices()
				.nth(character)
				.map_or(line.len(), |(byte, _)| byte),
		}
	}

	/// Converts a [`Span`] into a [`Range`] in `source`.
	pub fn range_in(self, source: &str, span: Span) -> Range {
		Range {
			start: self.position_in(source, span.start),
			end: self.position_in(source, span.end),
		}
	}

	fn position_in(self, source: &str, offset: u32) -> Position {
		let mut offset = (offset as usize).min(source.len());
		while offset > 0 && !source.is_char_boundary(offset) {
			offset -= 1;
		}
		let prefix = &source[..offset];
		let line_start =
			memrchr(b'\n', prefix.as_bytes()).map_or(0, |newline| newline + 1);
		Position {
			line: u32::try_from(bytecount::count(prefix.as_bytes(), b'\n'))
				.unwrap_or(u32::MAX),
			character: self.byte_to_column(&prefix[line_start..]),
		}
	}

	/// The `character` column at the end of `prefix`, which is the part of a
	/// line before the position being converted.
	fn byte_to_column(self, prefix: &str) -> u32 {
		let column = match self {
			Self::Utf8 => prefix.len(),
			Self::Utf16 => prefix
				.chars()
				.map(char::len_utf16)
				.sum(),
			Self::Utf32 => prefix.chars().count(),
		};
		u32::try_from(column).unwrap_or(u32::MAX)
	}
}

/// An open text document
/// 
/// resolving offset -> position is `O(log n)`
#[derive(Clone)]
pub struct Document {
	/// The content of the opened text document.
	pub text: String,
	/// The version number of this document (it will increase after each
	/// change, including undo/redo).
	version: i32,
	/// Byte offset of the first byte of every line, in ascending order.
	///
	/// Always starts with `0`, so this is never empty and its length is the
	/// number of lines in the document.
	line_starts: Vec<u32>,
	/// Copy of [`Inner::encoding`]
	encoding: PositionEncoding,
}

impl Document {
	fn new(item: TextDocumentItem, encoding: PositionEncoding) -> Self {
		let mut doc = Self {
			text: item.text,
			version: item.version,
			line_starts: Vec::new(),
			encoding,
		};
		doc.reindex();
		doc
	}

	/// re-creates [`Self::line_starts`] from the current text,
	/// recycling the allocation
	fn reindex(&mut self) {
		self.line_starts.clear();
		self.line_starts
			.reserve(self.text.len() / 64 + 1);
		self.line_starts.push(0);
		self.line_starts.extend(
			memchr_iter(b'\n', self.text.as_bytes())
				.map(|index| u32::try_from(index + 1).unwrap_or(u32::MAX)),
		);
	}

	fn len(&self) -> u32 {
		u32::try_from(self.text.len()).unwrap_or(u32::MAX)
	}

	/// The byte offset one past the last byte of `line`, not counting the
	/// trailing newline. A `line` past the end of the document gives the
	/// length of the document.
	fn line_end(&self, line: u32) -> u32 {
		self.line_starts
			.get(line as usize + 1)
			// every line but the last one ends with the single byte `\n`
			.map_or_else(|| self.len(), |next_start| next_start - 1)
	}

	/// Returns the byte offset of a **0-based** [`Position`], whose
	/// [`character`](Position::character) is read with [`Self::encoding`].
	///
	/// A line past the end of the document is clamped to the end of the
	/// document; see [`PositionEncoding::column_to_byte`] for how the column
	/// is clamped.
	pub fn offset_at(&self, position: Position) -> u32 {
		let Some(&start) = self
			.line_starts
			.get(position.line as usize)
		else {
			return self.len();
		};
		let line =
			&self.text[start as usize..self.line_end(position.line) as usize];
		let offset_in_line = self
			.encoding
			.column_to_byte(line, position.character);
		start + u32::try_from(offset_in_line).unwrap_or(u32::MAX)
	}

	/// Returns the **0-based** [`Position`] of a byte offset, whose
	/// `character` is written in the encoding negotiated with the client.
	///
	/// An offset past the end of the document is clamped to the end of the
	/// document, and one that falls inside a character is clamped back to
	/// where that character starts.
	pub fn position_at(&self, offset: u32) -> Position {
		let mut offset = (offset as usize).min(self.text.len());
		while offset > 0 && !self.text.is_char_boundary(offset) {
			offset -= 1;
		}
		// there is always a line starting at 0, so this never underflows
		let line = self
			.line_starts
			.partition_point(|&start| start as usize <= offset)
			- 1;
		let start = self.line_starts[line] as usize;
		Position {
			line: u32::try_from(line).unwrap_or(u32::MAX),
			character: self
				.encoding
				.byte_to_column(&self.text[start..offset]),
		}
	}

	/// Returns the [`Range`] a [`Span`] into this document covers.
	pub fn range_for_span(&self, span: Span) -> Range {
		Range {
			start: self.position_at(span.start),
			end: self.position_at(span.end),
		}
	}

	fn apply_changes(&mut self, changes: &[TextDocumentContentChangeEvent]) {
		for change in changes {
			if let Some(range) = &change.range {
				let start = self.offset_at(range.start) as usize;
				let end = self.offset_at(range.end) as usize;
				if start > end {
					warn!(
						start = start,
						end = end,
						"received change with an inverted range, skipping"
					);
					continue;
				}
				self.text
					.replace_range(start..end, &change.text);
			} else {
				debug!(
					"recieved change without range, replacing entire document"
				);
				self.text.clone_from(&change.text);
			}
			// the edit moved every line after it, so the index has to be
			// rebuilt before the next change is resolved against it
			self.reindex();
		}
	}
}

impl Files {
	/// Picks the position encoding to use out of the ones the client offers,
	/// and returns the kind to report back in the server's capabilities.
	///
	/// Prefers UTF-8, then UTF-32, and finally UTF-16
	#[instrument(skip_all, fields(client_encodings = ?capabilities.and_then(|c| c.position_encodings.as_deref())))]
	pub fn negotiate_encoding(
		&self,
		capabilities: Option<&GeneralClientCapabilities>,
	) -> PositionEncodingKind {
		let offered = capabilities
			.and_then(|c| c.position_encodings.as_deref())
			.unwrap_or_default();
		let encoding = if offered.contains(&PositionEncodingKind::UTF8) {
			PositionEncoding::Utf8
		} else if offered.contains(&PositionEncodingKind::UTF32) {
			PositionEncoding::Utf32
		} else {
			// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#serverCapabilities
			// > If the client didn't provide any position encodings the only valid value that a server can return is 'utf-16'.
			PositionEncoding::Utf16
		};
		if self.0.encoding.set(encoding).is_err() {
			warn!(
				"received a second initialize request, keeping the encoding negotiated by the first"
			);
		}
		let encoding = self.encoding();
		info!(?encoding, "negotiated position encoding");
		encoding.kind()
	}

	pub fn encoding(&self) -> PositionEncoding {
		self.0
			.encoding
			.get()
			.copied()
			.unwrap_or_default()
	}

	pub fn get(&self, uri: &Url) -> Option<Arc<Document>> {
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
		self.0.cache.insert(
			params.text_document.uri.clone(),
			Arc::new(Document::new(params.text_document, self.encoding())),
		);
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
			let doc = Arc::make_mut(&mut doc);
			doc.apply_changes(&changes);
			doc.version = new.version;
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

#[cfg(test)]
mod tests;
