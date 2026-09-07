use crate::{Sealed, base::WebpackChunkParserImpl};
use anyhow::{Result, anyhow};
use ast_parser::{
	exts::{ExpressionExt, MemberExpressionExt, StatementExt},
	parse_no_sema,
};
use oxc::{
	allocator::Allocator,
	ast::ast::{CallExpression, Expression, ObjectExpression, Program},
	span::{GetSpan as _, SourceType, Span},
};
use parser_diag::{PResult, err, slice_span};
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
		let prog = parse_no_sema(alloc, source_text, SourceType::script())
			.map_err(|e| anyhow!(e))?;
		Ok(Self {
			source_text,
			prog: alloc.alloc(prog),
		})
	}
	fn get_push_call(&self) -> PResult<&'ast CallExpression<'ast>> {
		let top_level_stmts = &self.prog.body;

		// we only expect one top-level statement
		if top_level_stmts.len() != 1 {
			return Err(err(
				&slice_span(top_level_stmts).unwrap_or(self.prog.span),
				"expected one top-level statement",
			));
		}

		let top_level_expr_stmt = &top_level_stmts[0]
			.as_expression_statement()
			.ok_or_else(|| {
				err(
					&top_level_stmts[0],
					"top level statement is not an expression statement",
				)
			})?
			.expression;
		let call = top_level_expr_stmt
			.as_call_expression()
			.ok_or_else(|| {
				err(
					top_level_expr_stmt,
					"top level statement is not a call expression",
				)
			})?;
		if call.arguments.len() != 1 {
			return Err(err(
				&slice_span(&call.arguments).unwrap_or_else(|| {
					Span::new(call.callee.span().end, call.span.end)
				}),
				"expected push call to have exactly one argument",
			));
		}

		// ensure push call
		let callee_sme = call
			.callee
			.as_static_member_expression()
			.ok_or_else(|| {
				err(
					&call.callee,
					"push call callee is not a static member expression",
				)
			})?;
		if callee_sme.property.name != "push" {
			return Err(err(
				&callee_sme.property,
				"push callee property is not `push`",
			));
		}

		Ok(call)
	}
	fn assert_one_entry(
		&self,
	) -> PResult<(&'ast Expression<'ast>, &'ast Expression<'ast>)> {
		let push_call = self.get_push_call()?;
		let first_arg = push_call
			.arguments
			.first()
			.expect("Get push call asserts one argument");
		let elements = &first_arg
			.as_array_expression()
			.ok_or_else(|| {
				err(first_arg, "first arugment is not an array expression")
			})?
			.elements;
		if elements.len() != 2 {
			return Err(err(
				&slice_span(elements).unwrap_or_else(|| first_arg.span()),
				"expected push call array to have exactly two elements",
			));
		}
		let a = elements[0]
			.as_expression()
			.ok_or_else(|| {
				err(
					&elements[0],
					"push call arg array elements must be expressions",
				)
			})?;
		let b = elements[1]
			.as_expression()
			.ok_or_else(|| {
				err(
					&elements[1],
					"push call arg array elements must be expressions",
				)
			})?;

		Ok((a, b))
	}

	pub fn chunk_id(&self) -> PResult<Cow<'ast, str>> {
		let (chunk_ids_expr, _) = self.assert_one_entry()?;
		let chunk_ids = &chunk_ids_expr
			.as_array_expression()
			.ok_or_else(|| {
				err(chunk_ids_expr, "push_call.arguments.0.0 is not an array")
			})?
			.elements;
		if chunk_ids.len() != 1 {
			return Err(err(
				chunk_ids_expr,
				"push_call.arguments.0.0 is expected to have exactly one element",
			));
		}
		let id_raw = chunk_ids.first().unwrap();
		id_raw
			.try_parse_string_or_number_literal()
			.ok_or_else(|| err(id_raw, "Failed to parse as string or number"))
	}
}

impl Sealed for WebpackLazyChunkParser<'_> {}

impl<'ast> WebpackChunkParserImpl<'ast> for WebpackLazyChunkParser<'ast> {
	fn get_module_object(&self) -> PResult<&'ast ObjectExpression<'ast>> {
		let (_, modules_arg) = self.assert_one_entry()?;

		modules_arg
			.as_object_expression()
			.ok_or_else(|| {
				err(
					modules_arg,
					"push_call.arguments.0.1 is not an object expression",
				)
			})
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
