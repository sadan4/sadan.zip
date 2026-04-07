//! Private utilities for [`super::WebpackAstParser`].
#![deny(clippy::missing_docs_in_private_items)]
use ast_parser::exts::{ExpressionExt, Functionish, StatementExt as _};
use oxc::ast::ast::{
	ArrowFunctionExpression,
	Expression,
	IdentifierName,
	IdentifierReference,
	MemberExpression,
	StaticMemberExpression,
};

use crate::parser::export_map::ExportMapKey;

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
