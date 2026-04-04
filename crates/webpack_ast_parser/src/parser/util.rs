//! Private utilities for [`super::WebpackAstParser`].
#![deny(clippy::missing_docs_in_private_items)]
use ast_parser::exts::{ExpressionExt, Functionish, StatementExt as _};
use oxc::ast::ast::{ArrowFunctionExpression, Expression, IdentifierReference};

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
