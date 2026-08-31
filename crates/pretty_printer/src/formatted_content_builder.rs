mod rope;

use std::hint::likely;

use derive_more::Debug;
use oxc::allocator::Allocator;
use unicode_ident::is_xid_continue;

use crate::{
	formatted_content_builder::rope::Rope,
	indent_cache::INDENT_CACHE,
};

#[derive(Debug)]
/// if `indent_size` is 0, tabs will be used
pub struct FormattedContentBuilder<'s> {
	#[debug(skip)]
	formatted_content: Rope<'s>,
	indent_size: u8,
	nesting_level: u32,
	new_lines: u32,
	enforce_space_before_words: bool,
	soft_space: bool,
	hard_spaces: u32,
	last_original_position: u32,
	last_formatted_position: u32,
	mappings: Vec<(u32, u32)>,
}

impl<'a> FormattedContentBuilder<'a> {
	pub fn new(alloc: &'a Allocator, indent_size: u8) -> Self {
		Self {
			formatted_content: Rope::new_in(alloc),
			indent_size,
			nesting_level: 0,
			new_lines: 0,
			enforce_space_before_words: true,
			soft_space: false,
			hard_spaces: 0,
			last_formatted_position: 0,
			last_original_position: 0,
			mappings: vec![(0, 0)],
		}
	}

	pub const fn set_enforce_space_between_words(
		&mut self,
		value: bool,
	) -> bool {
		let old_value = self.enforce_space_before_words;
		self.enforce_space_before_words = value;
		old_value
	}

	pub(crate) fn hint_num_tokens(&mut self, n: usize) {
		self.mappings.reserve_exact(n);
		// TODO: good margin for rope?
		self.formatted_content.reserve(n);
	}

	pub fn add_token(&mut self, token: &'a str, original_position: u32) {
		// Skip the regex check if `addSoftSpace` would be a no-op
		if self.enforce_space_before_words
			&& self.hard_spaces == 0
			&& !self.soft_space
		{
			// Only the first char of the new token matters here — that is
			// the boundary where token merging would actually happen.
			// Using `.any` would spuriously space tokens like `} width=${`
			// (template middle) when they trail an identifier.
			if let Some(last_char_of_last_token) =
				self.formatted_content.last_char()
				&& is_valid_ident_char(last_char_of_last_token)
				&& token
					.chars()
					.next()
					.is_some_and(is_valid_ident_char)
			{
				self.add_soft_space();
			}
		}
		self.append_formatting();
		// Insert token.
		self.add_mapping_if_needed(original_position);
		self.add_text(token);
	}

	pub const fn add_soft_space(&mut self) {
		if self.hard_spaces == 0 {
			self.soft_space = true;
		}
	}
	pub const fn add_hard_space(&mut self) {
		self.soft_space = false;
		self.hard_spaces += 1;
	}
	pub fn add_new_line(&mut self, no_squash: Option<bool>) {
		let no_squash = no_squash.unwrap_or(false);
		// Avoid leading newlines.
		if self.formatted_content.is_empty() {
			return;
		}
		if no_squash {
			self.new_lines += 1;
		} else {
			self.new_lines = if self.new_lines == 0 {
				1
			} else {
				self.new_lines
			};
		}
	}
	pub const fn increase_nesting_level(&mut self) {
		self.nesting_level += 1;
	}
	pub const fn decrease_nesting_level(&mut self) {
		if self.nesting_level != 0 {
			self.nesting_level -= 1;
		}
	}
	#[cfg(test)]
	/// TODO: move to impl in [`tests`] module
	fn build(self) -> String {
		self.build_with_mappings().code
	}
	pub fn build_with_mappings(mut self) -> crate::FormattedContent {
		if self.new_lines != 0 {
			self.formatted_content.push("\n");
		}
		let txt = self.formatted_content.to_string();
		let mappings = self.mappings;
		crate::FormattedContent {
			code: txt,
			mappings,
		}
	}
	fn append_formatting(&mut self) {
		if self.new_lines != 0 {
			for _ in 0..self.new_lines {
				self.add_text("\n");
			}
			'done: {
				if (self.indent_size as usize) < INDENT_CACHE.len() {
					// SAFETY: we just checked that self.indent_size is a valid index
					let indent_cache = unsafe {
						INDENT_CACHE.get_unchecked(self.indent_size as usize)
					};
					if likely(
						(self.nesting_level as usize) < indent_cache.len(),
					) {
						// SAFETY: we just checked that it's a valid index
						let str = unsafe {
							indent_cache
								.get_unchecked(self.nesting_level as usize)
						};
						self.add_text(str);
						break 'done;
					}
				}
				for _ in 0..self.nesting_level {
					if self.indent_size == 0 {
						self.add_text("\t");
					} else {
						for _ in 0..self.indent_size {
							self.add_text(" ");
						}
					}
				}
			}
		} else if self.soft_space {
			self.add_text(" ");
		}
		if self.hard_spaces != 0 {
			debug_assert!(!self.soft_space);
			for _ in 0..self.hard_spaces {
				self.add_text(" ");
			}
		}
		self.new_lines = 0;
		self.soft_space = false;
		self.hard_spaces = 0;
	}
	fn add_text(&mut self, text: &'a str) {
		self.formatted_content.push(text);
	}
	fn add_mapping_if_needed(&mut self, original_position: u32) {
		// formatted content length
		let fcl = self.formatted_content.len() as u32;
		if original_position - self.last_original_position
			== fcl - self.last_formatted_position
		{
			return;
		}
		let mapping = (original_position, fcl);
		self.last_original_position = original_position;
		self.last_formatted_position = fcl;
		self.mappings.push(mapping);
	}
}

fn is_valid_ident_char(c: char) -> bool {
	is_xid_continue(c) || c == '$'
}
#[cfg(test)]
mod tests {
	use super::*;

	use Allocator as A;
	use FormattedContentBuilder as F;

	#[test]
	fn zwj_and_zwnj_are_xid_continue() {
		const ZWJ: char = '\u{200D}';
		const ZWNJ: char = '\u{200C}';
		assert!(is_xid_continue(ZWJ));
		assert!(is_xid_continue(ZWNJ));
	}

	#[test]
	fn dollar_is_ident_char() {
		assert!(is_valid_ident_char('$'));
	}

	#[test]
	fn add_a_token() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		let src = "Test Script";
		b.add_token(src, 0);
		assert_eq!(b.build(), src);
	}

	#[test]
	fn returns_prev_enforce_spaces_value() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.set_enforce_space_between_words(false);
		assert!(!b.set_enforce_space_between_words(true));
	}

	#[test]
	fn squashes_new_lines_by_default() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_new_line(None);
		b.add_new_line(None);
		b.add_token("Token 2", 0);
		assert_eq!(b.build(), "Token 1\nToken 2");
	}

	#[test]
	fn respects_no_squash_parameter() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_new_line(None);
		b.add_new_line(Some(true));
		b.add_token("Token 2", 0);
		assert_eq!(b.build(), "Token 1\n\nToken 2");
	}

	#[test]
	fn avoids_leading_newlines() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_new_line(None);
		b.add_token("Token", 0);
		assert_eq!(b.build(), "Token");
	}

	#[test]
	fn avoids_more_than_one_trailing_newline() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token", 0);
		b.add_new_line(None);
		b.add_new_line(Some(true));
		assert_eq!(b.build(), "Token\n");
	}

	#[test]
	fn does_not_collapse_hard_spaces() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_hard_space();
		b.add_hard_space();
		b.add_hard_space();
		b.add_token("Token 2", 0);
		assert_eq!(b.build(), "Token 1   Token 2");
	}

	#[test]
	fn collapses_soft_spaces() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_soft_space();
		b.add_soft_space();
		b.add_soft_space();
		b.add_token("Token 2", 0);
		assert_eq!(b.build(), "Token 1 Token 2");
	}

	#[test]
	fn ignores_soft_space_after_hard_space() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_hard_space();
		b.add_soft_space();
		b.add_token("Token 2", 0);
		assert_eq!(b.build(), "Token 1 Token 2");
	}

	#[test]
	fn ignores_soft_space_before_hard_space() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_soft_space();
		b.add_hard_space();
		b.add_token("Token 2", 0);
		assert_eq!(b.build(), "Token 1 Token 2");
	}

	#[test]
	fn handles_nesting_levels() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("Token 1", 0);
		b.add_new_line(None);
		b.increase_nesting_level();
		b.add_token("Token 2", 0);
		b.add_new_line(None);
		b.increase_nesting_level();
		b.add_token("Token 3", 0);
		b.add_new_line(None);
		b.decrease_nesting_level();
		b.add_token("Token 4", 0);
		b.add_new_line(None);
		b.decrease_nesting_level();
		b.add_token("Token 5", 0);
		b.add_new_line(None);
		b.decrease_nesting_level();
		b.add_token("Token 6", 0);
		assert_eq!(
			b.build(),
			"Token 1\n  Token 2\n    Token 3\n  Token 4\nToken 5\nToken 6"
		);
	}

	#[test]
	fn formatted_source_mapping() {
		let a = A::new();
		let mut b = F::new(&a, 2);
		b.add_token("#main", 0);
		b.add_soft_space();
		b.add_token("{", 5);
		b.add_new_line(None);
		b.increase_nesting_level();
		b.add_token("color", 6);
		b.add_token(":", 11);
		b.add_soft_space();
		b.add_token("red", 13);
		b.add_token(";", 16);
		b.add_new_line(None);
		b.decrease_nesting_level();
		b.add_token("}", 17);
		b.add_new_line(None);

		let crate::FormattedContent {
			code: formatted_code,
			mappings: position_mappings,
		} = b.build_with_mappings();

		let (orig, fmt): (Vec<_>, Vec<_>) =
			position_mappings.into_iter().unzip();

		assert_eq!(orig, [0, 5, 6, 17]);
		assert_eq!(fmt, [0, 6, 10, 22]);
		assert_eq!(formatted_code, "#main {\n  color: red;\n}\n");
	}
}
