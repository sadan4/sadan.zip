use std::mem;

use derive_more::Debug;
use oxc::allocator::Allocator;

use crate::{
	formatted_content_builder::rope::Rope,
	unicode::{interval::interval_contains, tables::id_continue},
};

#[derive(Debug)]
/// if `INDENT_SIZE` is 0, tabs will be used
pub struct FormattedContentBuilder<'s, const INDENT_SIZE: usize = 4> {
	#[debug(skip)]
	formatted_content: Rope<'s>,
	nesting_level: usize,
	new_lines: usize,
	enforce_space_before_words: bool,
	soft_space: bool,
	hard_spaces: usize,
}

impl<'a, const INDENT_SIZE: usize> FormattedContentBuilder<'a, INDENT_SIZE> {
	const SPACES: &'static [u8] = &[b' '; INDENT_SIZE];
	const INDENT_STR: &'static str = if INDENT_SIZE == 0 {
		"\t"
	} else {
		unsafe {
			str::from_utf8_unchecked(
				FormattedContentBuilder::<'static, INDENT_SIZE>::SPACES,
			)
		}
	};
	pub fn new(_alloc: &'a Allocator) -> Self {
		Self {
			formatted_content: Rope::default(),
			nesting_level: 0,
			new_lines: 0,
			enforce_space_before_words: true,
			soft_space: false,
			hard_spaces: 0,
		}
	}

	pub const fn set_enforce_space_between_words(
		&mut self,
		value: bool,
	) -> bool {
		// mem::replace(&mut self.enforce_space_before_words, value)
		let old_value = self.enforce_space_before_words;
		self.enforce_space_before_words = value;
		old_value
	}

	pub fn add_token(&mut self, token: &'a str) {
		// Skip the regex check if `addSoftSpace` would be a no-op
		if self.enforce_space_before_words
			&& self.hard_spaces == 0
			&& !self.soft_space
		{
			//
			if let Some(last_char_of_last_token) =
				self.formatted_content.last_char()
				&& is_valid_ident_char(last_char_of_last_token)
				&& token.chars().any(is_valid_ident_char)
			{
				self.add_soft_space();
			}
		}
		self.append_formatting();
		// Insert token.
		// self.add_mapping_if_needed(offset);
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
	pub fn into_content(self) -> String {
		self.formatted_content.to_string()
	}
	fn append_formatting(&mut self) {
		if self.new_lines != 0 {
			for _ in 0..self.new_lines {
				self.add_text("\n");
			}
			for _ in 0..self.nesting_level {
				self.add_text(Self::INDENT_STR);
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
}

fn is_valid_ident_char(c: char) -> bool {
	const ZWNJ: u32 = '\u{200C}' as u32;
	const ZWJ: u32 = '\u{200D}' as u32;
	const ASCII_IDENT_START_TABLE: [bool; 128] = {
		let mut table = [false; 128];
		let mut i = 0;
		while i < 128 {
			table[i] = matches!(i as u8 as char, 'a'..='z' | 'A'..='Z' | '0'..='9' | '$' | '_');
			i += 1;
		}
		table
	};
	let c = c as u32;
	// TODO: is the lookup table worth it?
	if c < ASCII_IDENT_START_TABLE.len() as u32 {
		ASCII_IDENT_START_TABLE[c as usize]
	} else if matches!(c, ZWJ | ZWNJ) {
		true
	} else {
		interval_contains(id_continue(), c)
	}
}

mod rope {
	// TODO: would it be better to use a linked list over a vec
	// here since 99% of ops are appends?
	#[derive(Debug, Clone)]
	pub struct Rope<'s> {
		strs: Vec<&'s str>,
		total_len: usize,
	}

	impl Rope<'_> {
		pub const fn new() -> Self {
			Self {
				strs: Vec::new(),
				total_len: 0,
			}
		}
		pub fn last_char(&self) -> Option<char> {
			self.strs
				.last()
				.and_then(|s| s.chars().last())
		}

		pub const fn is_empty(&self) -> bool {
			self.total_len == 0
		}

		#[allow(clippy::inherent_to_string)]
		pub fn to_string(&self) -> String {
			let mut result = String::new();
			result.reserve_exact(self.total_len);
			for s in &self.strs {
				result.push_str(s);
			}
			debug_assert!(result.len() == self.total_len);
			result
		}
	}

	impl<'s> Rope<'s> {
		pub fn push(&mut self, s: &'s str) {
			self.total_len += s.len();
			self.strs.push(s);
		}
	}

	impl Default for Rope<'_> {
		fn default() -> Self {
			Self::new()
		}
	}
}
