//! Helpers for keeping tool output small and readable

use std::{fmt::Write as _, ops::Range};

use text_diff::{ContentDiff, DiffHunkKind};

/// Hard cap on the size of a single tool response, in bytes
pub const MAX_OUTPUT: usize = 40_000;

/// Cuts `s` down to [`MAX_OUTPUT`], appending `hint` to tell the caller how to
/// get the rest
pub fn truncate(mut s: String, hint: &str) -> String {
	if s.len() <= MAX_OUTPUT {
		return s;
	}
	let cut = s.floor_char_boundary(MAX_OUTPUT);
	s.truncate(cut);
	write!(s, "\n\n[output truncated at {MAX_OUTPUT} bytes. {hint}]").unwrap();
	s
}

/// Byte offsets of the start of every line
pub struct LineIndex {
	starts: Vec<usize>,
}

impl LineIndex {
	pub fn new(src: &str) -> Self {
		let mut starts = vec![0];
		starts.extend(
			src.bytes()
				.enumerate()
				.filter(|&(_, b)| b == b'\n')
				.map(|(i, _)| i + 1),
		);
		Self { starts }
	}

	/// 1-based line containing byte `pos`
	pub fn line(&self, pos: usize) -> usize {
		self.starts
			.partition_point(|&start| start <= pos)
	}

	pub const fn len(&self) -> usize {
		self.starts.len()
	}

	/// Byte range of the 1-based lines `start..=end`, clamped to the source
	pub fn range(
		&self,
		src_len: usize,
		start: usize,
		end: usize,
	) -> Range<usize> {
		let start = start.clamp(1, self.len());
		let end = end.clamp(start, self.len());
		let from = self.starts[start - 1];
		let to = self
			.starts
			.get(end)
			.copied()
			.unwrap_or(src_len);
		from..to
	}
}

/// `context` bytes either side of `range`, on one line
pub fn snippet(src: &str, range: Range<usize>, context: usize) -> String {
	let start = src.floor_char_boundary(range.start.saturating_sub(context));
	let end = src.ceil_char_boundary((range.end + context).min(src.len()));
	let mut out = String::with_capacity(end - start + 2);
	if start > 0 {
		out.push('…');
	}
	out.extend(
		src[start..end]
			.chars()
			.map(|c| match c {
				'\n' | '\r' | '\t' => ' ',
				c => c,
			}),
	);
	if end < src.len() {
		out.push('…');
	}
	out
}

/// The first line of `src[range]`, cut down to `max` bytes
pub fn first_line(src: &str, range: Range<usize>, max: usize) -> &str {
	let text = src.get(range).unwrap_or_default();
	let text = text
		.lines()
		.next()
		.unwrap_or_default()
		.trim();
	&text[..text.floor_char_boundary(max)]
}

/// Prefixes each line of `src` with its 1-based line number, starting at
/// `first_line`
pub fn number_lines(src: &str, first_line: usize) -> String {
	let mut out = String::with_capacity(src.len() + src.len() / 8);
	for (i, line) in src.lines().enumerate() {
		writeln!(out, "{:>5}  {line}", first_line + i).unwrap();
	}
	out
}

/// A line based diff of `old` against `new`, with `context` unchanged lines
/// around each change
pub fn unified_diff(old: &str, new: &str, context: usize) -> String {
	let diff = ContentDiff::by_line([old, new]);
	let mut out = String::new();
	let mut old_line = 1;
	let hunks = diff.hunks().collect::<Vec<_>>();
	let last = hunks.len().saturating_sub(1);
	for (i, hunk) in hunks.iter().enumerate() {
		let old_text = String::from_utf8_lossy(hunk.contents[0]);
		let old_lines = old_text.lines().collect::<Vec<_>>();
		match hunk.kind {
			DiffHunkKind::Matching => {
				let n = old_lines.len();
				let head = if i == 0 { 0 } else { context.min(n) };
				let tail = if i == last { 0 } else { context.min(n) };
				if head + tail >= n {
					for line in &old_lines {
						writeln!(out, " {line}").unwrap();
					}
				} else {
					for line in &old_lines[..head] {
						writeln!(out, " {line}").unwrap();
					}
					if tail > 0 {
						writeln!(out, "@@ line {} @@", old_line + n - tail)
							.unwrap();
					}
					for line in &old_lines[n - tail..] {
						writeln!(out, " {line}").unwrap();
					}
				}
				old_line += n;
			}
			DiffHunkKind::Different => {
				for line in &old_lines {
					writeln!(out, "-{line}").unwrap();
				}
				let new_text = String::from_utf8_lossy(hunk.contents[1]);
				for line in new_text.lines() {
					writeln!(out, "+{line}").unwrap();
				}
				old_line += old_lines.len();
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn line_index() {
		let src = "a\nbc\n\nd";
		let idx = LineIndex::new(src);
		assert_eq!(idx.len(), 4);
		assert_eq!(idx.line(0), 1);
		assert_eq!(idx.line(2), 2);
		assert_eq!(idx.line(5), 3);
		assert_eq!(idx.line(6), 4);
		assert_eq!(&src[idx.range(src.len(), 2, 3)], "bc\n\n");
		assert_eq!(&src[idx.range(src.len(), 4, 99)], "d");
	}

	#[test]
	fn diff_shows_changes_with_context() {
		let old = "1\n2\n3\n4\n5\n6\n7\n8\n";
		let new = "1\n2\n3\n4\nfive\n6\n7\n8\n";
		let diff = unified_diff(old, new, 1);
		assert!(diff.contains("-5\n+five\n"), "{diff}");
		assert!(diff.contains(" 4\n"), "{diff}");
		assert!(diff.contains(" 6\n"), "{diff}");
		assert!(!diff.contains(" 1\n"), "{diff}");
	}

	#[test]
	fn truncates_long_output() {
		let s = "x".repeat(MAX_OUTPUT + 10);
		let out = truncate(s, "narrow it");
		assert!(out.len() < MAX_OUTPUT + 100);
		assert!(out.ends_with("narrow it]"));
	}
}
