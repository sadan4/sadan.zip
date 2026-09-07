#![feature(likely_unlikely)]
use anyhow::Result;
use oxc::allocator::Allocator;

use parser_diag::ParserDiagnostic;

use crate::{
	formatted_content_builder::FormattedContentBuilder,
	javascript_formatter::JavaScriptFormatter,
};

mod formatted_content_builder;
mod indent_cache;
mod javascript_formatter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormattedContent {
	/// The formatted code.
	pub code: String,
	/// `Vec<(original_position, formatted_position)>`
	pub mappings: Vec<(u32, u32)>,
}

impl FormattedContent {
	/// The position in [`Self::code`] that `original` (a position in the
	/// source this was formatted from) maps to.
	#[must_use]
	pub fn map_pos(&self, original: u32) -> u32 {
		map_pos(&self.mappings, original)
	}
}

/// `mappings` must be sorted in ascending original position order, which is
/// how [`FormattedContent::mappings`] is built.
#[must_use]
pub fn map_pos(mappings: &[(u32, u32)], original: u32) -> u32 {
	let index = mappings.partition_point(|&(before, _)| before <= original);

	let Some(&(before, after)) = index
		.checked_sub(1)
		.and_then(|i| mappings.get(i))
	else {
		return 0;
	};

	after + (original - before)
}

/// Renders `e` against a pretty printed copy of `source`, remapping its spans
/// into the formatted text so the labelled source is readable.
///
/// Falls back to rendering against `source` itself when it cannot be
/// formatted.
#[must_use]
pub fn render_diag(e: ParserDiagnostic, source: &str, name: &str) -> String {
	match format(source, 4) {
		Ok(fmt) => format!(
			"{:?}",
			e.remap_spans(|pos| fmt.map_pos(pos))
				.with_local_source(&fmt.code, name)
		),
		Err(_) => format!("{:?}", e.with_local_source(source, name)),
	}
}

pub fn format_to_str(source: &str, indent_size: u8) -> Result<String> {
	format(source, indent_size).map(|c| c.code)
}

pub fn format(source: &str, indent_size: u8) -> Result<FormattedContent> {
	let alloc = Allocator::new();
	format_with_alloc(source, &alloc, indent_size)
}

pub fn format_with_alloc(
	source: &str,
	alloc: &Allocator,
	indent_size: u8,
) -> Result<FormattedContent> {
	let builder = FormattedContentBuilder::new(indent_size);
	JavaScriptFormatter::run(alloc, builder, source)
}
