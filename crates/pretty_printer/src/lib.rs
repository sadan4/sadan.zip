use anyhow::Result;
use oxc::allocator::Allocator;

use crate::{
	formatted_content_builder::FormattedContentBuilder,
	javascript_formatter::JavaScriptFormatter,
};

mod formatted_content_builder;
mod javascript_formatter;
mod unicode;

pub fn format(source: &str, indent_size: usize) -> Result<String> {
	let alloc = Allocator::new();
	format_with_alloc(source, &alloc, indent_size)
}

pub fn format_with_alloc(
	source: &str,
	alloc: &Allocator,
	indent_size: usize,
) -> Result<String> {
	let builder = FormattedContentBuilder::new(alloc, indent_size);
	JavaScriptFormatter::run(alloc, builder, source)
}
