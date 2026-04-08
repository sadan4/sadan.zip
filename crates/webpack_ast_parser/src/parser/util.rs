//! Private utilities for [`super::WebpackAstParser`].
#![deny(clippy::missing_docs_in_private_items)]
use std::{iter, mem};

use ast_parser::exts::{ExpressionExt, Functionish, StatementExt as _};
use itertools::Itertools as _;
use oxc::ast::ast::{
	ArrowFunctionExpression,
	Expression,
	IdentifierName,
	IdentifierReference,
	StaticMemberExpression,
};

use crate::parser::export_map::{
	ExportMapKey,
	RangeExportMap,
	RangeExportMapValue,
	RangeExportRange,
};

use super::export_map::{ExportMapEntry, ExportValue};

/// Given a function like below, return `x`
/// ```js
/// function foo() {
///     return x;
/// }
/// const bar = () => x;
/// ```
/// Does not consider any early returns, only looks at the final return
pub fn find_return_identifier<'a, 'ast>(
	func: Functionish<'a, 'ast>,
) -> Option<&'a IdentifierReference<'ast>> {
	find_return_expr(func).and_then(Expression::as_identifier)
}

/// given a function like below, returns `a.b.c[d].#e`
/// ```js
/// function foo() {
///     return a.b.c[d].#e;
/// }
/// ```
/// Does not consider any early returns, only looks at the final return
pub fn find_return_member_expression<'a, 'ast>(
	func: Functionish<'a, 'ast>,
) -> Option<&'a oxc::ast::ast::MemberExpression<'ast>> {
	find_return_expr(func).and_then(Expression::as_member_expression)
}

// TODO: analysis with CFG
/// Given a function like below, return `expr`
/// ```js
/// function foo() {
///     return (((((expr)))));
/// }
/// const bar = () => expr;
/// const baz = () => {
///     return expr;
/// };
/// ```
/// Returns none in cases like
/// ```js
/// const foo = () => {};
/// ```
/// Does not consider any early returns, only looks at the final return
pub fn find_return_expr<'a, 'ast>(
	func: Functionish<'a, 'ast>,
) -> Option<&'a Expression<'ast>> {
	if let Some(expr) = func
		.as_arrow()
		.and_then(ArrowFunctionExpression::get_expression)
	{
		return Some(expr);
	}

	let ret = func
		.body()
		.statements
		.last()?
		.as_return_statement()?
		.argument
		.as_ref()?
		.get_inner_expression();

	Some(ret)
}

/// A property access chain
/// For example `foo().bar.baz` would be represented as `("foo()", ["bar", "baz"])`
/// See: [`flatten_property_access_expression`]
pub type PropertyAccessChain<'ast> =
	(&'ast Expression<'ast>, Vec<&'ast IdentifierName<'ast>>);

/// Flattens a property access expression, producing a [`PropertyAccessChain`].
///
/// For example, `foo.bar.baz[0].qux.abc` would be represented as `("foo.bar.baz[0]", ["qux", "abc"])`
///
/// `expr` must be the last element in the chain.
///
/// [`ast_parser::AstParser::last_parent`] can be used to find the outermost static member expression
pub fn flatten_property_access_expression<'ast>(
	mut expr: &'ast StaticMemberExpression<'ast>,
) -> PropertyAccessChain<'ast> {
	let mut ret = Vec::new();
	loop {
		ret.insert(0, &expr.property);
		match &expr.object {
			Expression::StaticMemberExpression(next) => expr = next,
			_ => break,
		}
	}
	(&expr.object, ret)
}

/// try to match `export_names` against `chain`
///
/// For example if `export_names` is `["foo", "bar"]` and the chain is `(module_ident, ["foo", "bar"])`, this will return Some("bar").
///
/// if `export_names` is `["foo", "bar", "baz", "qux"]` and the chain is `(module_ident, ["foo", "bar"])`, this will return None,
///
/// TODO: document how [`ExportMapKey::Default`] works with this
pub fn match_export_chain<'ast>(
	chain: &PropertyAccessChain<'ast>,
	export_names: &[ExportMapKey],
) -> Option<&'ast IdentifierName<'ast>> {
	let chain_v = &chain.1;
	if export_names.len() > chain_v.len() {
		return None;
	}

	let mut cur = None;
	for (export_name, chain_part) in export_names
		.iter()
		.zip(chain_v.iter().copied())
	{
		cur = Some(chain_part);
		match export_name {
			ExportMapKey::Named(name) => {
				if chain_part.name != name.as_str() {
					return None;
				}
			}
			ExportMapKey::Default => panic!("TODO: handle default export"),
		}
	}
	cur
}

/// Filter a given [`RangeExportMap`] to only include exports that include the range `pos`
pub fn filter_export_map(
	mut export_map: RangeExportMap,
	pos: u32,
) -> RangeExportMap {
	if let Some(cjs_def) = mem::take(&mut export_map.cjs_default) {
		let new_default = filter_export_value(*cjs_def, pos);
		if !new_default.is_empty() {
			export_map.cjs_default = Some(Box::new(new_default));
		}
	}

	export_map.exports = export_map
		.exports
		.into_iter()
		.filter_map(|(k, v)| {
			let new_v = filter_export_value(v, pos);
			if new_v.is_empty() {
				None
			} else {
				Some((k, new_v))
			}
		})
		.collect();
	export_map
}

/// Flattens a given [`RangeExportMap`] into a list of valid export keys
pub fn flatten_export_map(
	export_map: RangeExportMap,
	prefix: Option<&[ExportMapKey]>,
) -> Vec<Vec<ExportMapKey>> {
	let prefix = prefix.unwrap_or(&[]);
	export_map
		.into_iter()
		.flat_map(|ExportMapEntry(k, v)| {
			let current_path = prefix
				.iter()
				.cloned()
				.chain(iter::once(k))
				.collect_vec();
			debug_assert!(
				!v.is_empty(),
				"should have been filtered out by filter_export_map"
			);
			if let Ok(obj) = v.try_unwrap_map() {
				flatten_export_map(obj, Some(&current_path))
			} else {
				vec![current_path]
			}
		})
		.collect()
}

/// Filter a given [`RangeExportRange`] to only include exports that include the range `pos`
fn filter_export_range(
	mut export_range: RangeExportRange,
	pos: u32,
) -> RangeExportRange {
	if export_range.iter().any(|rng| pos >= rng.start && pos < rng.end) {
		export_range
	} else {
		RangeExportRange::default()
	}
}

/// Filter a given [`RangeExportMapValue`] to only include exports that include the range `pos`
fn filter_export_value(
	export_value: RangeExportMapValue,
	pos: u32,
) -> RangeExportMapValue {
	match export_value {
		ExportValue::Range(export_range) => {
			filter_export_range(export_range, pos).into()
		}
		ExportValue::Map(export_map) => {
			filter_export_map(export_map, pos).into()
		}
	}
}
