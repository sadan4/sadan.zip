use anyhow::Result;
use oxc::allocator::Allocator;

use crate::{
	formatted_content_builder::FormattedContentBuilder,
	javascript_formatter::JavaScriptFormatter,
};

mod formatted_content_builder;
mod javascript_formatter;
mod node_binder;
mod unicode;

pub fn format(source: &str) -> Result<String> {
	let alloc = Allocator::new();
	format_with_alloc(source, &alloc)
}

pub fn format_with_alloc(source: &str, alloc: &Allocator) -> Result<String> {
	let builder = FormattedContentBuilder::<4>::new(alloc);
	JavaScriptFormatter::run(alloc, builder, source)
}

pub fn format2(source: &str) -> Result<String> {
	let alloc = Allocator::new();
	format_with_alloc2(source, &alloc)
}

pub fn format_with_alloc2(source: &str, alloc: &Allocator) -> Result<String> {
	let builder = FormattedContentBuilder::<2>::new(alloc);
	JavaScriptFormatter::run(alloc, builder, source)
}
