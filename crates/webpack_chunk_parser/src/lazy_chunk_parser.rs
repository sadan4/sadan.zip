use crate::{Sealed, base::WebpackChunkParserImpl};
use anyhow::Result;
use ast_parser::{
	exts::{ExpressionExt, MemberExpressionExt, StatementExt},
	parse_no_sema,
};
use oxc::{
	allocator::Allocator,
	ast::ast::{CallExpression, Expression, ObjectExpression, Program},
	span::SourceType,
};
use std::borrow::Cow;

// TODO: should we cache things here
pub struct WebpackLazyChunkParser<'ast> {
	source_text: &'ast str,
	prog: &'ast Program<'ast>,
}

impl<'ast> WebpackLazyChunkParser<'ast> {
	pub fn try_new(
		alloc: &'ast Allocator,
		source_text: &'ast str,
	) -> Result<Self> {
		let prog = parse_no_sema(alloc, source_text, SourceType::script())?;
		Ok(Self {
			source_text,
			prog: alloc.alloc(prog),
		})
	}
	fn get_push_call(&self) -> Option<&'ast CallExpression<'ast>> {
		let top_level_stmts = &self.prog.body;

		// we only expect one top-level statement
		if top_level_stmts.len() != 1 {
			return None;
		}

		let call = top_level_stmts[0]
			.as_expression_statement()?
			.expression
			.as_call_expression()?;
		if call.arguments.len() != 1 {
			return None;
		}

		// ensure push call
		if call
			.callee
			.as_static_member_expression()?
			.property
			.name != "push"
		{
			return None;
		}

		Some(call)
	}
	fn assert_one_entry(
		&self,
	) -> Option<(&'ast Expression<'ast>, &'ast Expression<'ast>)> {
		let elements = &self
			.get_push_call()?
			.arguments
			.first()?
			.as_array_expression()?
			.elements;
		if elements.len() != 2 {
			return None;
		}
		let a = elements[0].as_expression()?;
		let b = elements[1].as_expression()?;

		Some((a, b))
	}
	// TODO: should this be a Option<u32>
	pub fn chunk_id(&self) -> Option<Cow<'ast, str>> {
		self.assert_one_entry()?
			.0
			.as_array_expression()?
			.elements
			.first()?
			.try_parse_string_or_number_literal()
	}
}

impl Sealed for WebpackLazyChunkParser<'_> {}

impl<'ast> WebpackChunkParserImpl<'ast> for WebpackLazyChunkParser<'ast> {
	fn get_module_object(&self) -> Option<&'ast ObjectExpression<'ast>> {
		let modules_arg = self
			.assert_one_entry()?
			.1
			.as_object_expression()?;

		Some(modules_arg)
	}

	fn get_source_text(&self) -> &'ast str {
		self.source_text
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::base::WebpackChunkParser;
	use insta::assert_ron_snapshot;
	use itertools::Itertools;
	macro_rules! parse {
		($alloc:expr, $source:literal) => {{
			let source = include_str!($source);
			WebpackLazyChunkParser::try_new(&$alloc, source).unwrap()
		}};
	}
	mod old_format {
		use super::*;

		#[test]
		fn gets_modules_from_a_lazy_chunk() {
			let alloc = Allocator::new();
			let parser = parse!(alloc, "test_data/lazyChunk.js");
			// there is some form of random state that makes this non-deterministic, collect into a sorted vec
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_iter()
				.sorted_by_key(|item| item.0)
				.collect_vec();
			assert_ron_snapshot!(modules);
		}
		#[test]
		fn gets_modules_from_an_i18n_chunk() {
			let alloc = Allocator::new();
			let parser = parse!(alloc, "test_data/lazyChunk-i18n.js");
			// there is some form of random state that makes this non-deterministic, collect into a sorted vec
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_iter()
				.sorted_by_key(|item| item.0)
				.collect_vec();

			assert_ron_snapshot!(modules);
		}
		#[test]
		fn gets_chunk_id_from_a_lazy_chunk() {
			let alloc = Allocator::new();
			let parser = parse!(alloc, "test_data/lazyChunk.js");
			let chunk_id = &parser.chunk_id().unwrap();
			assert_eq!(chunk_id, r"24314");
		}
	}
	mod new_format {
		use super::*;

		#[test]
		fn gets_modules_from_a_lazy_chunk() {
			let alloc = Allocator::new();
			let parser = parse!(alloc, "test_data/lazyChunk2.js");
			// there is some form of random state that makes this non-deterministic, collect into a sorted vec
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_iter()
				.sorted_by_key(|item| item.0)
				.collect_vec();
			assert_ron_snapshot!(modules);
		}
		#[test]
		fn gets_modules_from_an_i18n_chunk() {
			let alloc = Allocator::new();
			let parser = parse!(alloc, "test_data/lazyChunk2-i18n.js");
			// there is some form of random state that makes this non-deterministic, collect into a sorted vec
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_iter()
				.sorted_by_key(|item| item.0)
				.collect_vec();
			assert_ron_snapshot!(modules);
		}
		#[test]
		fn gets_chunk_id_from_a_lazy_chunk() {
			let alloc = Allocator::new();
			let parser = parse!(alloc, "test_data/lazyChunk2.js");
			let chunk_id = &parser.chunk_id().unwrap();
			assert_eq!(chunk_id, r"52694");
		}
	}
}
