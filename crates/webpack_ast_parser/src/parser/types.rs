#![deny(clippy::missing_docs_in_private_items)]
//! Private types for [`super::WebpackAstParser`]
use ast_parser::ast_kind::IntoAstKind;
use derive_more::{From, Into, TryInto, Unwrap};
use oxc::ast::{
	AstKind,
	ast::{
		CallExpression,
		Expression,
		IdentifierReference,
		MemberExpression,
		ObjectExpression,
	},
};

/// `wreq.d(exports, { foo: () => local_foo })`
#[derive(Copy, Clone, Debug)]
pub struct WreqD<'ast> {
	/// the entire call expression
	pub call: &'ast CallExpression<'ast>,
	/// `exports` in the above example
	pub exports: &'ast IdentifierReference<'ast>,
	/// `{ foo: () => local_foo }` in the above example
	pub obj: &'ast ObjectExpression<'ast>,
}

/// A value that can represent val in `wreq.d(exports, { foo: () => /*val*/ })`
#[derive(Clone, Copy, Debug, From)]
pub enum WreqDExportType<'ast> {
	/// `local_foo` in `wreq.d(exports, { foo: () => local_foo })`
	Ident(&'ast IdentifierReference<'ast>),
	/// `a.b` in `wreq.d(exports, { foo: () => a.b })`
	/// Most commonly used with re-exports such as
	/// `wreq.d(exports, { foo: () => otherModule.foo })`
	Access(&'ast MemberExpression<'ast>),
}

impl<'ast> TryFrom<&'ast Expression<'ast>> for WreqDExportType<'ast> {
	type Error = &'ast Expression<'ast>;

	fn try_from(value: &'ast Expression<'ast>) -> Result<Self, Self::Error> {
		match value {
			Expression::Identifier(e) => Ok(e.as_ref().into()),
			e @ (Expression::ComputedMemberExpression(_)
			| Expression::StaticMemberExpression(_)
			| Expression::PrivateFieldExpression(_)) => {
				Ok(e.as_member_expression().unwrap().into())
			}
			e => Err(e),
		}
	}
}

impl<'ast> Into<AstKind<'ast>> for WreqDExportType<'ast> {
	fn into(self) -> oxc::ast::AstKind<'ast> {
		match self {
			WreqDExportType::Ident(ident) => ident.into_ast_kind(),
			WreqDExportType::Access(mem_expr) => mem_expr.into_ast_kind(),
		}
	}
}

impl<'ast> IntoAstKind<'ast> for WreqDExportType<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		self.into()
	}
}
