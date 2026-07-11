#![deny(clippy::missing_docs_in_private_items)]
//! Private types for [`super::WebpackAstParser`]
use std::rc::Rc;

use ast_parser::{ast_kind::IntoAstKind, exts::MemberExprAccessKind};
use derive_more::From;
use explorer_types::ModuleId;
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

use crate::{WebpackAstParser, parser::export_map::ExportMapKey};

/// `wreq.d(exports, { foo: () => local_foo })`
#[derive(Copy, Clone, Debug)]
pub struct WreqD<'ast> {
	/// the entire call expression
	pub _call: &'ast CallExpression<'ast>,
	/// `exports` in the above example
	pub _exports: &'ast IdentifierReference<'ast>,
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

impl<'ast> From<WreqDExportType<'ast>> for AstKind<'ast> {
	fn from(val: WreqDExportType<'ast>) -> Self {
		match val {
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

/// Helper type for [`super::WebpackAstParser::generate_references`]
#[derive(Debug)]
pub struct SearchElement {
	/// The module id that is being searched for uses
	pub module_id: ModuleId,
	/// The id of the module that [`Self::export_name`] will be imported from
	pub imported_id: ModuleId,
	/// The exported name to search
	pub export_name: Vec<ExportMapKey>,
}

/// Helper type for [`WebpackAstParser::does_re_export_from_export`]
pub struct ReExport<'ast> {
	/// TODO: doc
	pub import_source_id: ModuleId,
	/// TODO: doc
	pub export_names: Vec<MemberExprAccessKind<'ast>>,
}

/// A definition resolved from a position.
///
/// Used to abstract logic from position/hover queries.
pub struct ResolvedDefinition<'ast> {
	/// the parser that has the definition
	pub parser: Rc<WebpackAstParser<'ast>>,
	/// the chain of export names to get the definition from [`Self::parser`]
	pub raw_export_names: Vec<MemberExprAccessKind<'ast>>,
	/// the chain of export names to get the definition from [`Self::parser`]
	pub export_names: Vec<ExportMapKey>,
}
