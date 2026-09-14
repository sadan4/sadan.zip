use ast_parser::{get_line_and_column, get_offset_from_line_and_column};
use tower_lsp::lsp_types::{
	DidOpenTextDocumentParams,
	GeneralClientCapabilities,
	Position,
	PositionEncodingKind,
	Range,
	TextDocumentContentChangeEvent,
	TextDocumentItem,
	Url,
};

use oxc::span::Span;

use super::{Document, Files, PositionEncoding};

/// `a` is 1 byte, `é` is 2, and `😀` is 4 (and a surrogate pair in
/// UTF-16), so the encodings disagree about every column past the start
/// of a line.
const MIXED: &str = "aé\n😀z";

fn item(text: &str) -> TextDocumentItem {
	TextDocumentItem {
		uri: Url::parse("file:///test.js").unwrap(),
		language_id: String::from("javascript"),
		version: 0,
		text: String::from(text),
	}
}

fn doc_in(encoding: PositionEncoding, text: &str) -> Document {
	Document::new(item(text), encoding)
}

fn doc(text: &str) -> Document {
	doc_in(PositionEncoding::Utf8, text)
}

fn pos(line: u32, character: u32) -> Position {
	Position { line, character }
}

fn offering(encodings: &[PositionEncodingKind]) -> GeneralClientCapabilities {
	GeneralClientCapabilities {
		position_encodings: Some(encodings.to_vec()),
		..GeneralClientCapabilities::default()
	}
}

#[test]
fn indexes_the_start_of_every_line() {
	assert_eq!(doc("").line_starts, [0]);
	assert_eq!(doc("abc").line_starts, [0]);
	assert_eq!(doc("a\nb").line_starts, [0, 2]);
	assert_eq!(doc("a\n\nb\n").line_starts, [0, 2, 3, 5]);
}

#[test]
fn utf8_columns_count_bytes() {
	let doc = doc(MIXED);

	assert_eq!(doc.position_at(3), pos(0, 3));
	assert_eq!(doc.offset_at(pos(0, 3)), 3);
	assert_eq!(doc.position_at(4), pos(1, 0));
	assert_eq!(doc.position_at(8), pos(1, 4));
	assert_eq!(doc.offset_at(pos(1, 4)), 8);
}

#[test]
fn utf16_columns_count_code_units() {
	let doc = doc_in(PositionEncoding::Utf16, MIXED);

	assert_eq!(doc.position_at(3), pos(0, 2));
	assert_eq!(doc.offset_at(pos(0, 2)), 3);
	assert_eq!(doc.position_at(4), pos(1, 0));
	// the emoji is a surrogate pair, so `z` sits at column 2
	assert_eq!(doc.position_at(8), pos(1, 2));
	assert_eq!(doc.offset_at(pos(1, 2)), 8);
}

#[test]
fn clamps_columns_that_split_a_character() {
	// the emoji covers bytes 4..8, which is columns 0..4 of line 1 in
	// UTF-8 and columns 0..2 of line 1 in UTF-16
	assert_eq!(doc(MIXED).offset_at(pos(1, 2)), 4);
	assert_eq!(
		doc_in(PositionEncoding::Utf16, MIXED).offset_at(pos(1, 1)),
		4
	);
	assert_eq!(doc(MIXED).position_at(6), pos(1, 0));
}

#[test]
fn clamps_positions_past_the_end() {
	let doc = doc("ab\nc");

	assert_eq!(doc.offset_at(pos(0, 99)), 2);
	assert_eq!(doc.offset_at(pos(99, 0)), 4);
	assert_eq!(doc.position_at(u32::MAX), pos(1, 1));
}

#[test]
fn round_trips_every_character_boundary() {
	for encoding in [
		PositionEncoding::Utf8,
		PositionEncoding::Utf16,
		PositionEncoding::Utf32,
	] {
		for text in ["", "abc", "ab\nc", "a\n\nb\n", MIXED, "a\r\nb"] {
			let doc = doc_in(encoding, text);
			let boundaries = (0..=text.len())
				.filter(|&offset| text.is_char_boundary(offset));
			for offset in boundaries {
				let offset = u32::try_from(offset).unwrap();
				assert_eq!(
					doc.offset_at(doc.position_at(offset)),
					offset,
					"{encoding:?} round trip of {offset} in {text:?}"
				);
			}
		}
	}
}

/// UTF-32 columns are Unicode scalar values, which is what the linear
/// scan in `ast_parser` counts, so the two have to agree exactly.
#[test]
fn utf32_agrees_with_the_linear_scan() {
	for text in ["", "abc", "a\nb", "ab\nc", "a\n\nb\n", MIXED, "a\r\nb"] {
		let doc = doc_in(PositionEncoding::Utf32, text);
		for line in 0..=doc.line_starts.len() as u32 + 1 {
			for character in 0..=text.chars().count() as u32 + 1 {
				assert_eq!(
					doc.offset_at(pos(line, character)),
					get_offset_from_line_and_column(text, line, character),
					"offset of {line}:{character} in {text:?}"
				);
			}
		}
		for offset in 0..=text.len() as u32 + 1 {
			let (line, character) = get_line_and_column(text, offset);
			assert_eq!(
				doc.position_at(offset),
				pos(line, character),
				"position of {offset} in {text:?}"
			);
		}
	}
}

/// [`PositionEncoding::range_in`] has no line index to lean on, so it
/// has to arrive at the same answer the hard way.
#[test]
fn converts_spans_in_unindexed_text() {
	for encoding in [
		PositionEncoding::Utf8,
		PositionEncoding::Utf16,
		PositionEncoding::Utf32,
	] {
		for text in ["", "abc", "ab\nc", "a\n\nb\n", MIXED, "a\r\nb"] {
			let doc = doc_in(encoding, text);
			let end = u32::try_from(text.len()).unwrap();
			for start in (0..=end)
				.filter(|&o| text.is_char_boundary(usize::try_from(o).unwrap()))
			{
				let span = Span::new(start, end);
				assert_eq!(
					encoding.range_in(text, span),
					doc.range_for_span(span),
					"{encoding:?} range of {span:?} in {text:?}"
				);
			}
		}
	}
}

#[test]
fn reindexes_after_an_incremental_change() {
	let mut doc = doc("one\ntwo\nthree");

	doc.apply_changes(&[TextDocumentContentChangeEvent {
		range: Some(Range {
			start: pos(0, 3),
			end: pos(1, 3),
		}),
		range_length: None,
		text: String::from("\nand a\nhalf\n"),
	}]);

	assert_eq!(doc.text, "one\nand a\nhalf\n\nthree");
	assert_eq!(doc.line_starts, [0, 4, 10, 15, 16]);
	assert_eq!(doc.offset_at(pos(2, 0)), 10);
	assert_eq!(doc.position_at(16), pos(4, 0));
}

#[test]
fn replaces_the_whole_document_for_a_rangeless_change() {
	let mut doc = doc("one\ntwo");

	doc.apply_changes(&[TextDocumentContentChangeEvent {
		range: None,
		range_length: None,
		text: String::from("just one line"),
	}]);

	assert_eq!(doc.text, "just one line");
	assert_eq!(doc.line_starts, [0]);
}

#[test]
fn applies_changes_in_order() {
	let mut doc = doc("a\nb");

	doc.apply_changes(&[
		TextDocumentContentChangeEvent {
			range: Some(Range {
				start: pos(1, 0),
				end: pos(1, 1),
			}),
			range_length: None,
			text: String::from("bbb\nc"),
		},
		// resolved against the document the first change produced
		TextDocumentContentChangeEvent {
			range: Some(Range {
				start: pos(2, 1),
				end: pos(2, 1),
			}),
			range_length: None,
			text: String::from("cc"),
		},
	]);

	assert_eq!(doc.text, "a\nbbb\nccc");
}

#[test]
fn negotiates_utf8_when_the_client_offers_it() {
	let files = Files::default();

	assert_eq!(
		files.negotiate_encoding(Some(&offering(&[
			PositionEncodingKind::UTF16,
			PositionEncodingKind::UTF8,
		]))),
		PositionEncodingKind::UTF8
	);
	assert_eq!(files.encoding(), PositionEncoding::Utf8);
}

#[test]
fn falls_back_to_utf16_for_a_client_that_offers_nothing() {
	let files = Files::default();

	assert_eq!(files.negotiate_encoding(None), PositionEncodingKind::UTF16);
	assert_eq!(files.encoding(), PositionEncoding::Utf16);
}

#[test]
fn negotiates_utf32_over_mandatory_utf16() {
	let files = Files::default();

	assert_eq!(
		files.negotiate_encoding(Some(&offering(&[
			PositionEncodingKind::UTF16,
			PositionEncodingKind::UTF32,
		]))),
		PositionEncodingKind::UTF32
	);
	assert_eq!(files.encoding(), PositionEncoding::Utf32);
}

#[test]
fn keeps_the_first_negotiated_encoding() {
	let files = Files::default();

	files.negotiate_encoding(Some(&offering(&[PositionEncodingKind::UTF8])));

	assert_eq!(
		files.negotiate_encoding(Some(&offering(&[
			PositionEncodingKind::UTF16,
		]))),
		PositionEncodingKind::UTF8
	);
}

#[test]
fn opens_documents_with_the_negotiated_encoding() {
	let files = Files::default();
	let uri = Url::parse("file:///test.js").unwrap();
	files.negotiate_encoding(Some(&offering(&[PositionEncodingKind::UTF8])));

	files.handle_open(DidOpenTextDocumentParams {
		text_document: item(MIXED),
	});

	assert_eq!(files.get(&uri).unwrap().position_at(3), pos(0, 3));
}
