//! Private types for [`super::WebpackAstParser`]
use oxc::ast::ast::{CallExpression, IdentifierReference, ObjectExpression};

/// `wreq.d(exports, { foo: () => local_foo })`
pub struct WreqD<'ast> {
	pub call: &'ast CallExpression<'ast>,
	pub exports: &'ast IdentifierReference<'ast>,
	pub obj: &'ast ObjectExpression<'ast>,
}
