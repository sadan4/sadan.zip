use anyhow::Result;
use oxc::allocator::Allocator;

use crate::{
	formatted_content_builder::FormattedContentBuilder,
	javascript_formatter::JavaScriptFormatter,
};

mod formatted_content_builder;
mod indent_cache;
mod javascript_formatter;
mod unicode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormattedContent {
	/// The formatted code.
	pub code: String,
	/// `Vec<(original_position, formatted_position)>`
	pub mappings: Vec<(u32, u32)>,
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
	let builder = FormattedContentBuilder::new(alloc, indent_size);
	JavaScriptFormatter::run(alloc, builder, source)
}
