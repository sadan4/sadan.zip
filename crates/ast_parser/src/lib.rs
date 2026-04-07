//! TODO: Document this crate.
#![warn(missing_docs)]
pub mod ast_kind;
mod ast_parser;
pub mod exts;
pub mod sym_id;

pub use ast_parser::{
	AstParser,
	ESModuleParser,
	parse,
	parse_for_traverse,
	parse_no_sema,
};
use oxc::span::Span;

/// Returns the **0-based** line and column for a byte offset in `source`.
///
/// The `offset` is interpreted as a byte index. If it is larger than
/// `source.len()`, it is clamped to the end of the string. If it falls inside a
/// UTF-8 code point, it is clamped backward to the nearest valid character
/// boundary.
pub fn get_line_and_column(source: &str, offset: u32) -> (u32, u32) {
	let mut clamped: usize = (offset as usize).min(source.len());
	while clamped > 0 && !source.is_char_boundary(clamped) {
		clamped -= 1;
	}

	let mut line: u32 = 0;
	let mut column: u32 = 0;

	for ch in source[..clamped].chars() {
		if ch == '\n' {
			line += 1;
			column = 0;
		} else {
			column += 1;
		}
	}

	(line, column)
}

pub fn span_line_and_column(
	source: &str,
	span: Span,
) -> ((u32, u32), (u32, u32)) {
	let start = get_line_and_column(source, span.start);
	let end = get_line_and_column(source, span.end);
	(start, end)
}

#[cfg(test)]
mod tests {
	use super::get_line_and_column;

	#[test]
	fn returns_start_position_for_zero_offset() {
		assert_eq!(get_line_and_column("abc", 0), (0, 0));
	}

	#[test]
	fn tracks_column_with_ascii_offsets() {
		assert_eq!(get_line_and_column("abc", 1), (0, 1));
		assert_eq!(get_line_and_column("abc", 2), (0, 2));
		assert_eq!(get_line_and_column("abc", 3), (0, 3));
	}

	#[test]
	fn resets_column_after_newline() {
		let source: &str = "a\nb";

		assert_eq!(get_line_and_column(source, 1), (0, 1));
		assert_eq!(get_line_and_column(source, 2), (1, 0));
		assert_eq!(get_line_and_column(source, 3), (1, 1));
	}

	#[test]
	fn clamps_offsets_that_fall_inside_utf8_codepoints() {
		let source: &str = "aé\n😀z";

		// Offset 2 points into the middle of 'é' (2-byte codepoint), so it clamps to 1.
		assert_eq!(get_line_and_column(source, 2), (0, 1));
		// Offset 7 points into the middle of '😀' (4-byte codepoint), so it clamps to 4.
		assert_eq!(get_line_and_column(source, 7), (1, 0));
	}

	#[test]
	fn clamps_offsets_beyond_source_length() {
		assert_eq!(get_line_and_column("ab\nc", u32::MAX), (1, 1));
	}
}
