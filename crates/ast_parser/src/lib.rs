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

/// Returns the byte offset for a **0-based** `(line, column)` in `source`.
///
/// The `column` is measured in Unicode scalar values (`char`s), not bytes.
/// If `line` is past the last line, the result is clamped to `source.len()`.
/// If `column` is past the target line's end, it is clamped to the line end
/// (or to the newline byte for non-final lines).
pub fn get_offset_from_line_and_column(
	source: &str,
	line: u32,
	column: u32,
) -> u32 {
	let target_line: u32 = line;
	let target_column: u32 = column;

	let mut current_line: u32 = 0;
	let mut line_start: usize = 0;

	for (index, ch) in source.char_indices() {
		if current_line == target_line {
			break;
		}

		if ch == '\n' {
			current_line += 1;
			line_start = index + ch.len_utf8();
		}
	}

	if current_line < target_line {
		return u32::try_from(source.len()).unwrap_or(u32::MAX);
	}

	if target_column == 0 {
		return u32::try_from(line_start).unwrap_or(u32::MAX);
	}

	let mut current_column: u32 = 0;
	for (relative_index, ch) in source[line_start..].char_indices() {
		if ch == '\n' {
			return u32::try_from(line_start + relative_index)
				.unwrap_or(u32::MAX);
		}

		current_column += 1;
		if current_column == target_column {
			return u32::try_from(line_start + relative_index + ch.len_utf8())
				.unwrap_or(u32::MAX);
		}
	}

	u32::try_from(source.len()).unwrap_or(u32::MAX)
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
	use super::{get_line_and_column, get_offset_from_line_and_column};

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

	#[test]
	fn returns_zero_offset_for_zero_line_and_column() {
		assert_eq!(get_offset_from_line_and_column("abc", 0, 0), 0);
	}

	#[test]
	fn converts_ascii_line_and_column_to_offsets() {
		assert_eq!(get_offset_from_line_and_column("abc", 0, 1), 1);
		assert_eq!(get_offset_from_line_and_column("abc", 0, 2), 2);
		assert_eq!(get_offset_from_line_and_column("abc", 0, 3), 3);
	}

	#[test]
	fn converts_positions_around_newlines() {
		let source: &str = "a\nb";

		assert_eq!(get_offset_from_line_and_column(source, 0, 1), 1);
		assert_eq!(get_offset_from_line_and_column(source, 1, 0), 2);
		assert_eq!(get_offset_from_line_and_column(source, 1, 1), 3);
	}

	#[test]
	fn uses_character_columns_for_utf8() {
		let source: &str = "aé\n😀z";

		assert_eq!(get_offset_from_line_and_column(source, 0, 2), 3);
		assert_eq!(get_offset_from_line_and_column(source, 1, 1), 8);
	}

	#[test]
	fn clamps_columns_and_lines_past_the_end() {
		let source: &str = "ab\nc";

		assert_eq!(get_offset_from_line_and_column(source, 0, 99), 2);
		assert_eq!(get_offset_from_line_and_column(source, 1, 99), 4);
		assert_eq!(get_offset_from_line_and_column(source, 99, 0), 4);
	}

	#[test]
	fn round_trips_valid_positions() {
		let source: &str = "aé\n😀z";

		let positions: [(u32, u32); 6] =
			[(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)];

		for (line, column) in positions {
			let offset: u32 =
				get_offset_from_line_and_column(source, line, column);
			assert_eq!(get_line_and_column(source, offset), (line, column));
		}
	}
}
