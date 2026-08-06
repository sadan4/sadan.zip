use anyhow::Result;
use derive_more::{From, TryUnwrap};
use itertools::Itertools as _;
use oxc::{
	allocator::Box as OxcBox, ast::{
		AstKind, ast::{
			Argument, ArrayExpression, ArrayExpressionElement, ArrowFunctionBody, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget, BigIntLiteral, BinaryExpression, BindingIdentifier, BindingPattern, CallExpression, ComputedMemberExpression, ConditionalExpression, Expression, ExpressionStatement, Function, FunctionBody, IdentifierName, IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, MemberExpression, ModuleDeclaration, NumericLiteral, ObjectExpression, ObjectProperty, PrivateFieldExpression, PrivateIdentifier, PropertyKey, ReturnStatement, SequenceExpression, SpreadElement, Statement, StaticMemberExpression, Str, StringLiteral, TaggedTemplateExpression, TemplateLiteral,
		},
	}, semantic::{NodeId, ScopeId, SymbolId}, span::{GetSpan, Span},
};
use oxc_ecmascript::{GlobalContext, constant_evaluation::IsLiteralValue};
use std::borrow::Cow;

use crate::ast_kind::IntoAstKind;

pub trait ModuleDeclarationExt {
	fn as_import_declaration(
		&'_ self,
	) -> Option<&'_ OxcBox<'_, ImportDeclaration<'_>>>;
}

impl ModuleDeclarationExt for ModuleDeclaration<'_> {
	fn as_import_declaration(
		&'_ self,
	) -> Option<&'_ OxcBox<'_, ImportDeclaration<'_>>> {
		match self {
			ModuleDeclaration::ImportDeclaration(i) => Some(i),
			_ => None,
		}
	}
}

pub trait ImportDeclarationExt {
	// fn default_or_namespace_var(&self) -> Option<SymbolId>;
	// fn namespace_var(&self) -> Option<SymbolId>;
	fn default_var(&self) -> Option<SymbolId>;
	// fn get_imported_var(&self, name: &str) -> Option<SymbolId>;
}

impl ImportDeclarationExt for ImportDeclaration<'_> {
	// fn get_imported_var(&self, name: &str) -> Option<SymbolId> {
	//     let specifiers = self.specifiers.as_ref()?;
	//     for spec in specifiers {
	//         if let ImportDeclarationSpecifier::ImportSpecifier(i) = spec
	//             && i.imported.name() == name
	//         {
	//             return Some(i.local.symbol_id());
	//         }
	//     }
	//     None
	// }
	fn default_var(&self) -> Option<SymbolId> {
		for spec in self.specifiers.as_ref()? {
			if let ImportDeclarationSpecifier::ImportDefaultSpecifier(i) = spec
			{
				return Some(i.local.symbol_id());
			}
		}
		None
	}
	// fn namespace_var(&self) -> Option<SymbolId> {
	//     for spec in self.specifiers.as_ref()? {
	//         if let ImportDeclarationSpecifier::ImportNamespaceSpecifier(i) = spec {
	//             return Some(i.local.symbol_id());
	//         }
	//     }
	//     None
	// }
	// fn default_or_namespace_var(&self) -> Option<SymbolId> {
	//     for spec in self.specifiers.as_ref()? {
	//         match spec {
	//             ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
	//             | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
	//                 return Some(spec.local().symbol_id());
	//             }
	//             ImportDeclarationSpecifier::ImportSpecifier(_) => {}
	//         }
	//     }
	//     None
	// }
}

pub trait ObjectExpressionExt {
	fn get_property<'a>(&'a self, name: &str)
	-> Option<&'a ObjectProperty<'a>>;
	/// try to get a boolean literal property from an object
	/// returns Ok(false) if not present
	/// returns Err(expr) if present but not a bool
	fn parse_bool_flag<'a>(
		&'a self,
		key: &str,
	) -> Result<bool, &'a Expression<'a>> {
		self.get_property(key)
			.map_or(Ok(false), |prop| match &prop.value {
				Expression::BooleanLiteral(b) => Ok(b.value),
				other => Err(other),
			})
	}
}

impl ObjectExpressionExt for ObjectExpression<'_> {
	fn get_property<'a>(
		&'a self,
		name: &str,
	) -> Option<&'a ObjectProperty<'a>> {
		for prop in &self.properties {
			let Some(prop) = prop.as_property() else {
				continue;
			};
			match &prop.key {
				PropertyKey::StaticIdentifier(i) if i.name == name => {
					return Some(prop);
				}
				PropertyKey::NumericLiteral(num)
					if num.value.to_string() == name =>
				{
					return Some(prop);
				}
				PropertyKey::StringLiteral(s) if s.value == name => {
					return Some(prop);
				}
				_ => {}
			}
		}
		None
	}
}

pub trait PropertyKeyExt<'ast>: ExpressionExt<'ast> {
	fn as_prop_key_(&self) -> Option<&PropertyKey<'ast>>;
	fn as_prop_key_mut_(&mut self) -> Option<&mut PropertyKey<'ast>>;
	fn as_static_identifier(&self) -> Option<&IdentifierName<'ast>> {
		match self.as_prop_key_()? {
			PropertyKey::StaticIdentifier(i) => Some(i.as_ref()),
			_ => None,
		}
	}
	fn as_static_identifier_mut(
		&mut self,
	) -> Option<&mut IdentifierName<'ast>> {
		match self.as_prop_key_mut_()? {
			PropertyKey::StaticIdentifier(i) => Some(i.as_mut()),
			_ => None,
		}
	}
	fn as_private_identifier(&self) -> Option<&PrivateIdentifier<'ast>> {
		match self.as_prop_key_()? {
			PropertyKey::PrivateIdentifier(i) => Some(i.as_ref()),
			_ => None,
		}
	}
	fn as_private_identifier_mut(
		&mut self,
	) -> Option<&mut PrivateIdentifier<'ast>> {
		match self.as_prop_key_mut_()? {
			PropertyKey::PrivateIdentifier(i) => Some(i.as_mut()),
			_ => None,
		}
	}
}

impl<'ast> ExpressionExt<'ast> for PropertyKey<'ast> {
	fn as_expr_(&self) -> Option<&Expression<'ast>> {
		self.as_expression()
	}

	fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>> {
		self.as_expression_mut()
	}
	fn dbg_name(&self) -> &'static str {
		match self {
			PropertyKey::StaticIdentifier(_) => "StaticIdentifier",
			PropertyKey::PrivateIdentifier(_) => "PrivateIdentifier",
			// should never error
			_ => self.as_expr_().unwrap().dbg_name(),
		}
	}
}

impl<'ast> PropertyKeyExt<'ast> for PropertyKey<'ast> {
	fn as_prop_key_(&self) -> Option<&Self> {
		Some(self)
	}

	fn as_prop_key_mut_(&mut self) -> Option<&mut Self> {
		Some(self)
	}
}

pub trait ArgumentExt<'ast>: ExpressionExt<'ast> {
	fn as_arg_(&self) -> Option<&Argument<'ast>>;
	fn as_arg_mut_(&mut self) -> Option<&mut Argument<'ast>>;
	fn as_spread(&self) -> Option<&SpreadElement<'ast>> {
		match self.as_arg_()? {
			Argument::SpreadElement(s) => Some(s.as_ref()),
			_ => None,
		}
	}
	fn as_spread_mut(&mut self) -> Option<&mut SpreadElement<'ast>> {
		match self.as_arg_mut_()? {
			Argument::SpreadElement(s) => Some(s.as_mut()),
			_ => None,
		}
	}
}

impl<'ast> ExpressionExt<'ast> for Argument<'ast> {
	fn as_expr_(&self) -> Option<&Expression<'ast>> {
		self.as_expression()
	}

	fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>> {
		self.as_expression_mut()
	}

	fn dbg_name(&self) -> &'static str {
		match self {
			Argument::SpreadElement(_) => "SpreadElement",
			_ => self.as_expr_().unwrap().dbg_name(),
		}
	}
}

impl<'ast> ArgumentExt<'ast> for Argument<'ast> {
	fn as_arg_(&self) -> Option<&Self> {
		Some(self)
	}

	fn as_arg_mut_(&mut self) -> Option<&mut Self> {
		Some(self)
	}
}

pub trait StatementExt<'ast> {
	fn as_stmt_(&self) -> Option<&Statement<'ast>>;
	fn as_stmt_mut_(&mut self) -> Option<&mut Statement<'ast>>;

	fn is_expression_statement(&self) -> bool {
		matches!(self.as_stmt_(), Some(Statement::ExpressionStatement(_)))
	}
	fn as_expression_statement(&self) -> Option<&ExpressionStatement<'ast>> {
		match self.as_stmt_()? {
			Statement::ExpressionStatement(s) => Some(s.as_ref()),
			_ => None,
		}
	}
	fn as_expression_statement_mut(
		&mut self,
	) -> Option<&mut ExpressionStatement<'ast>> {
		match self.as_stmt_mut_()? {
			Statement::ExpressionStatement(s) => Some(s.as_mut()),
			_ => None,
		}
	}

	fn is_return_statement(&self) -> bool {
		matches!(self.as_stmt_(), Some(Statement::ReturnStatement(_)))
	}
	fn as_return_statement(&self) -> Option<&ReturnStatement<'ast>> {
		match self.as_stmt_()? {
			Statement::ReturnStatement(s) => Some(s.as_ref()),
			_ => None,
		}
	}
	fn as_return_statement_mut(
		&mut self,
	) -> Option<&mut ReturnStatement<'ast>> {
		match self.as_stmt_mut_()? {
			Statement::ReturnStatement(s) => Some(s.as_mut()),
			_ => None,
		}
	}
}

impl<'ast> StatementExt<'ast> for Statement<'ast> {
	fn as_stmt_(&self) -> Option<&Self> {
		Some(self)
	}

	fn as_stmt_mut_(&mut self) -> Option<&mut Self> {
		Some(self)
	}
}

pub trait MemberExpressionExt<'ast> {
	fn as_member_expr_(&self) -> Option<&MemberExpression<'ast>>;
	fn as_member_expr_mut_(&mut self) -> Option<&mut MemberExpression<'ast>>;

	fn as_computed_member(&self) -> Option<&ComputedMemberExpression<'ast>> {
		match self.as_member_expr_()? {
			MemberExpression::ComputedMemberExpression(expr) => {
				Some(expr.as_ref())
			}
			_ => None,
		}
	}

	fn as_computed_member_mut(
		&mut self,
	) -> Option<&mut ComputedMemberExpression<'ast>> {
		match self.as_member_expr_mut_()? {
			MemberExpression::ComputedMemberExpression(expr) => {
				Some(expr.as_mut())
			}
			_ => None,
		}
	}

	fn as_static_member_expression(
		&self,
	) -> Option<&StaticMemberExpression<'ast>> {
		match self.as_member_expr_()? {
			MemberExpression::StaticMemberExpression(expr) => {
				Some(expr.as_ref())
			}
			_ => None,
		}
	}

	fn as_static_member_expression_mut(
		&mut self,
	) -> Option<&mut StaticMemberExpression<'ast>> {
		match self.as_member_expr_mut_()? {
			MemberExpression::StaticMemberExpression(expr) => {
				Some(expr.as_mut())
			}
			_ => None,
		}
	}

	fn as_private_field(&self) -> Option<&PrivateFieldExpression<'ast>> {
		match self.as_member_expr_()? {
			MemberExpression::PrivateFieldExpression(expr) => {
				Some(expr.as_ref())
			}
			_ => None,
		}
	}

	fn as_private_field_mut(
		&mut self,
	) -> Option<&mut PrivateFieldExpression<'ast>> {
		match self.as_member_expr_mut_()? {
			MemberExpression::PrivateFieldExpression(expr) => {
				Some(expr.as_mut())
			}
			_ => None,
		}
	}
}

impl<'ast> MemberExpressionExt<'ast> for MemberExpression<'ast> {
	fn as_member_expr_(&self) -> Option<&Self> {
		Some(self)
	}

	fn as_member_expr_mut_(&mut self) -> Option<&mut Self> {
		Some(self)
	}
}

impl<'ast> MemberExpressionExt<'ast> for AssignmentTarget<'ast> {
	fn as_member_expr_(&self) -> Option<&MemberExpression<'ast>> {
		self.as_member_expression()
	}

	fn as_member_expr_mut_(&mut self) -> Option<&mut MemberExpression<'ast>> {
		self.as_member_expression_mut()
	}
}

impl<'ast, T: ExpressionExt<'ast>> MemberExpressionExt<'ast> for T {
	fn as_member_expr_(&self) -> Option<&MemberExpression<'ast>> {
		self.as_expr_()?.as_member_expression()
	}

	fn as_member_expr_mut_(&mut self) -> Option<&mut MemberExpression<'ast>> {
		self.as_expr_mut_()?
			.as_member_expression_mut()
	}
}

#[derive(Debug, Copy, Clone)]
pub enum Functionish<'a, 'ast> {
	Named(&'a Function<'ast>),
	Arrow(&'a ArrowFunctionExpression<'ast>),
}

impl<'a, 'ast> From<&'a ArrowFunctionExpression<'ast>>
	for Functionish<'a, 'ast>
{
	fn from(v: &'a ArrowFunctionExpression<'ast>) -> Self {
		Self::Arrow(v)
	}
}

impl<'a, 'ast> From<&'a Function<'ast>> for Functionish<'a, 'ast> {
	fn from(v: &'a Function<'ast>) -> Self {
		Self::Named(v)
	}
}

impl<'a, 'ast> Functionish<'a, 'ast> {
	/// Gets the identifier of this function, if it has one
	/// See: [`Function::id`]
	pub fn id(&self) -> Option<&'a BindingIdentifier<'ast>> {
		self.as_named()
			.and_then(|f| f.id.as_ref())
	}
	/// See [`Function::pife`]
	/// See [`ArrowFunctionExpression::pife`]
	pub const fn pife(&self) -> bool {
		match self {
			Self::Arrow(ArrowFunctionExpression { pife, .. })
			| Self::Named(Function { pife, .. }) => *pife,
		}
	}
	/// See [`Function::pure`]
	/// See [`ArrowFunctionExpression::pure`]
	pub const fn pure(&self) -> bool {
		match self {
			Self::Arrow(ArrowFunctionExpression { pure, .. })
			| Self::Named(Function { pure, .. }) => *pure,
		}
	}
	/// See [`Function::async`]
	/// See [`ArrowFunctionExpression::async`]
	pub const fn r#async(&self) -> bool {
		match self {
			Self::Arrow(ArrowFunctionExpression { r#async, .. })
			| Self::Named(Function { r#async, .. }) => *r#async,
		}
	}
	/// See [`Function::scope_id`]
	/// See [`ArrowFunctionExpression::scope_id`]
	pub const fn scope_id(&self) -> ScopeId {
		match self {
			Self::Arrow(ArrowFunctionExpression { scope_id, .. })
			| Self::Named(Function { scope_id, .. }) => scope_id.get().unwrap(),
		}
	}
	/// See [`Function::node_id`]
	/// See [`ArrowFunctionExpression::node_id`]
	pub const fn node_id(&self) -> NodeId {
		match self {
			Self::Arrow(ArrowFunctionExpression { node_id, .. })
			| Self::Named(Function { node_id, .. }) => node_id.get(),
		}
	}
	/// Panics if this is called on a function with no body (typescript declaration)
	/// 
	/// Returns none if called on an arrow function with an expression body
	pub fn body(&self) -> Option<&'a FunctionBody<'ast>> {
		match self {
			Self::Arrow(ArrowFunctionExpression { body, .. }) => match body {
				ArrowFunctionBody::FunctionBody(b) => Some(b.as_ref()),
				_ => None,
			},
			Self::Named(Function { body, .. }) => Some(body.as_ref().unwrap()),
		}
	}

	/// Returns `true` if the functionish is [`Named`].
	///
	/// [`Named`]: Functionish::Named
	#[must_use]
	pub const fn is_named(&self) -> bool {
		matches!(self, Self::Named(..))
	}

	pub const fn as_named(&self) -> Option<&'a Function<'ast>> {
		if let Self::Named(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub const fn try_into_named(self) -> Result<&'a Function<'ast>, Self> {
		if let Self::Named(v) = self {
			Ok(v)
		} else {
			Err(self)
		}
	}

	/// Returns `true` if the functionish is [`Arrow`].
	///
	/// [`Arrow`]: Functionish::Arrow
	#[must_use]
	pub const fn is_arrow(&self) -> bool {
		matches!(self, Self::Arrow(..))
	}

	pub const fn as_arrow(&self) -> Option<&'a ArrowFunctionExpression<'ast>> {
		if let Self::Arrow(v) = self {
			Some(v)
		} else {
			None
		}
	}

	pub const fn try_into_arrow(
		self,
	) -> Result<&'a ArrowFunctionExpression<'ast>, Self> {
		if let Self::Arrow(v) = self {
			Ok(v)
		} else {
			Err(self)
		}
	}
}

impl GetSpan for Functionish<'_, '_> {
	fn span(&self) -> Span {
		match self {
			Functionish::Named(f) => f.span(),
			Functionish::Arrow(a) => a.span(),
		}
	}
}

impl<'a> IntoAstKind<'a> for Functionish<'a, 'a> {
	fn into_ast_kind(self) -> AstKind<'a> {
		match self {
			Functionish::Named(f) => f.into_ast_kind(),
			Functionish::Arrow(a) => a.into_ast_kind(),
		}
	}
}

impl<'a> IntoAstKind<'a> for &Functionish<'a, 'a> {
	fn into_ast_kind(self) -> AstKind<'a> {
		(*self).into_ast_kind()
	}
}

pub trait ExpressionExt<'ast> {
	fn as_expr_(&self) -> Option<&Expression<'ast>>;
	fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>>;

	fn is_functionish(&self) -> bool {
		matches!(
			self.as_expr_(),
			Some(
				Expression::FunctionExpression(_)
					| Expression::ArrowFunctionExpression(_)
			)
		)
	}

	fn as_functionish(&self) -> Option<Functionish<'_, 'ast>> {
		match self.as_expr_()? {
			Expression::FunctionExpression(f) => {
				Some(Functionish::Named(f.as_ref()))
			}
			Expression::ArrowFunctionExpression(a) => {
				Some(Functionish::Arrow(a.as_ref()))
			}
			_ => None,
		}
	}

	fn as_numeric_literal(&self) -> Option<&NumericLiteral<'ast>> {
		match self.as_expr_()? {
			Expression::NumericLiteral(n) => Some(n),
			_ => None,
		}
	}
	fn as_numeric_literal_mut(&mut self) -> Option<&mut NumericLiteral<'ast>> {
		match self.as_expr_mut_()? {
			Expression::NumericLiteral(n) => Some(n),
			_ => None,
		}
	}

	fn as_big_int_literal(&self) -> Option<&BigIntLiteral<'ast>> {
		match self.as_expr_()? {
			Expression::BigIntLiteral(n) => Some(n),
			_ => None,
		}
	}

	fn as_big_int_literal_mut(&mut self) -> Option<&mut BigIntLiteral<'ast>> {
		match self.as_expr_mut_()? {
			Expression::BigIntLiteral(n) => Some(n),
			_ => None,
		}
	}

	fn as_function_expression(&self) -> Option<&Function<'ast>> {
		match self.as_expr_()? {
			Expression::FunctionExpression(f) => Some(f.as_ref()),
			_ => None,
		}
	}
	fn as_function_expression_mut(&mut self) -> Option<&mut Function<'ast>> {
		match self.as_expr_mut_()? {
			Expression::FunctionExpression(f) => Some(f.as_mut()),
			_ => None,
		}
	}

	fn as_arrow_function_expression(
		&self,
	) -> Option<&ArrowFunctionExpression<'ast>> {
		match self.as_expr_()? {
			Expression::ArrowFunctionExpression(f) => Some(f.as_ref()),
			_ => None,
		}
	}

	fn as_string_literal(&self) -> Option<&StringLiteral<'ast>> {
		match self.as_expr_()? {
			Expression::StringLiteral(s) => Some(s),
			_ => None,
		}
	}

	fn as_conditional_expression(
		&self,
	) -> Option<&ConditionalExpression<'ast>> {
		match self.as_expr_()? {
			Expression::ConditionalExpression(c) => Some(c.as_ref()),
			_ => None,
		}
	}

	fn as_conditional_expression_mut(
		&mut self,
	) -> Option<&mut ConditionalExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::ConditionalExpression(c) => Some(c.as_mut()),
			_ => None,
		}
	}

	fn as_object_expression(&self) -> Option<&ObjectExpression<'ast>> {
		match self.as_expr_()? {
			Expression::ObjectExpression(o) => Some(o.as_ref()),
			_ => None,
		}
	}

	fn as_array_expression(&self) -> Option<&ArrayExpression<'ast>> {
		match self.as_expr_()? {
			Expression::ArrayExpression(a) => Some(a.as_ref()),
			_ => None,
		}
	}
	fn as_array_expression_mut(
		&mut self,
	) -> Option<&mut ArrayExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::ArrayExpression(a) => Some(a.as_mut()),
			_ => None,
		}
	}

	fn as_call_expression(&self) -> Option<&CallExpression<'ast>> {
		match self.as_expr_()? {
			Expression::CallExpression(e) => Some(e.as_ref()),
			_ => None,
		}
	}
	fn as_call_expression_mut(&mut self) -> Option<&mut CallExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::CallExpression(e) => Some(e.as_mut()),
			_ => None,
		}
	}
	fn as_identifier(&self) -> Option<&IdentifierReference<'ast>> {
		match self.as_expr_()? {
			Expression::Identifier(i) => Some(i.as_ref()),
			_ => None,
		}
	}
	fn as_identifier_mut(&mut self) -> Option<&mut IdentifierReference<'ast>> {
		match self.as_expr_mut_()? {
			Expression::Identifier(i) => Some(i.as_mut()),
			_ => None,
		}
	}
	fn is_template_literal(&self) -> bool {
		matches!(self.as_expr_(), Some(Expression::TemplateLiteral(_)))
	}
	fn as_template_literal(&self) -> Option<&TemplateLiteral<'ast>> {
		match self.as_expr_()? {
			Expression::TemplateLiteral(i) => Some(i.as_ref()),
			_ => None,
		}
	}
	fn as_template_literal_mut(
		&mut self,
	) -> Option<&mut TemplateLiteral<'ast>> {
		match self.as_expr_mut_()? {
			Expression::TemplateLiteral(i) => Some(i.as_mut()),
			_ => None,
		}
	}
	fn as_tagged_template(&self) -> Option<&TaggedTemplateExpression<'ast>> {
		match self.as_expr_()? {
			Expression::TaggedTemplateExpression(e) => Some(e.as_ref()),
			_ => None,
		}
	}
	fn as_tagged_template_mut(
		&mut self,
	) -> Option<&mut TaggedTemplateExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::TaggedTemplateExpression(e) => Some(e.as_mut()),
			_ => None,
		}
	}
	fn as_binary_expression(&self) -> Option<&BinaryExpression<'ast>> {
		match self.as_expr_()? {
			Expression::BinaryExpression(e) => Some(e.as_ref()),
			_ => None,
		}
	}
	fn as_binary_expression_mut(
		&mut self,
	) -> Option<&mut BinaryExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::BinaryExpression(e) => Some(e.as_mut()),
			_ => None,
		}
	}

	fn is_sequence_expression(&self) -> bool {
		matches!(self.as_expr_(), Some(Expression::SequenceExpression(_)))
	}
	fn as_sequence_expression(&self) -> Option<&SequenceExpression<'ast>> {
		match self.as_expr_()? {
			Expression::SequenceExpression(e) => Some(e.as_ref()),
			_ => None,
		}
	}
	fn as_sequence_expression_mut(
		&mut self,
	) -> Option<&mut SequenceExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::SequenceExpression(e) => Some(e.as_mut()),
			_ => None,
		}
	}

	fn is_assignment_expression(&self) -> bool {
		matches!(self.as_expr_(), Some(Expression::AssignmentExpression(_)))
	}
	fn as_assignment_expression(&self) -> Option<&AssignmentExpression<'ast>> {
		match self.as_expr_()? {
			Expression::AssignmentExpression(e) => Some(e.as_ref()),
			_ => None,
		}
	}
	fn as_assignment_expression_mut(
		&mut self,
	) -> Option<&mut AssignmentExpression<'ast>> {
		match self.as_expr_mut_()? {
			Expression::AssignmentExpression(e) => Some(e.as_mut()),
			_ => None,
		}
	}

	fn as_string_literal_like(&self) -> Option<Str<'ast>> {
		match self.as_expr_()? {
			Expression::StringLiteral(s) => Some(s.value),
			Expression::TemplateLiteral(t)
				if t.is_no_substitution_template() =>
			{
				Some(t.quasis[0].value.cooked.unwrap())
			}
			_ => None,
		}
	}

	fn try_parse_string_or_number_literal(&self) -> Option<Cow<'ast, str>> {
		self.as_string_literal_like()
			.map(Into::into)
			.or_else(|| {
				self.as_numeric_literal().map(|n| {
					n.raw.map(Into::into).map_or_else(
						|| Cow::Owned(format!("{}", n.value)),
						Cow::Borrowed,
					)
				})
			})
	}

	fn dbg_name(&self) -> &'static str {
		match self.as_expr_().expect("unreachable") {
			Expression::BooleanLiteral(_) => "BooleanLiteral",
			Expression::NullLiteral(_) => "NullLiteral",
			Expression::NumericLiteral(_) => "NumericLiteral",
			Expression::BigIntLiteral(_) => "BigIntLiteral",
			Expression::RegExpLiteral(_) => "RegExpLiteral",
			Expression::StringLiteral(_) => "StringLiteral",
			Expression::TemplateLiteral(_) => "TemplateLiteral",
			Expression::Identifier(_) => "Identifier",
			Expression::ImportMeta(_) => "ImportMeta",
			Expression::NewTarget(_) => "NewTarget",
			Expression::Super(_) => "Super",
			Expression::ArrayExpression(_) => "ArrayExpression",
			Expression::ArrowFunctionExpression(_) => "ArrowFunctionExpression",
			Expression::AssignmentExpression(_) => "AssignmentExpression",
			Expression::AwaitExpression(_) => "AwaitExpression",
			Expression::BinaryExpression(_) => "BinaryExpression",
			Expression::CallExpression(_) => "CallExpression",
			Expression::ChainExpression(_) => "ChainExpression",
			Expression::ClassExpression(_) => "ClassExpression",
			Expression::ConditionalExpression(_) => "ConditionalExpression",
			Expression::FunctionExpression(_) => "FunctionExpression",
			Expression::ImportExpression(_) => "ImportExpression",
			Expression::LogicalExpression(_) => "LogicalExpression",
			Expression::NewExpression(_) => "NewExpression",
			Expression::ObjectExpression(_) => "ObjectExpression",
			Expression::ParenthesizedExpression(_) => "ParenthesizedExpression",
			Expression::SequenceExpression(_) => "SequenceExpression",
			Expression::TaggedTemplateExpression(_) => {
				"TaggedTemplateExpression"
			}
			Expression::ThisExpression(_) => "ThisExpression",
			Expression::UnaryExpression(_) => "UnaryExpression",
			Expression::UpdateExpression(_) => "UpdateExpression",
			Expression::YieldExpression(_) => "YieldExpression",
			Expression::PrivateInExpression(_) => "PrivateInExpression",
			Expression::JSXElement(_) => "JSXElement",
			Expression::JSXFragment(_) => "JSXFragment",
			Expression::TSAsExpression(_) => "TSAsExpression",
			Expression::TSSatisfiesExpression(_) => "TSSatisfiesExpression",
			Expression::TSTypeAssertion(_) => "TSTypeAssertion",
			Expression::TSNonNullExpression(_) => "TSNonNullExpression",
			Expression::TSInstantiationExpression(_) => {
				"TSInstantiationExpression"
			}
			Expression::V8IntrinsicExpression(_) => "V8IntrinsicExpression",
			Expression::ComputedMemberExpression(_) => {
				"ComputedMemberExpression"
			}
			Expression::StaticMemberExpression(_) => "StaticMemberExpression",
			Expression::PrivateFieldExpression(_) => "PrivateFieldExpression",
		}
	}
}

impl<'ast> ExpressionExt<'ast> for Expression<'ast> {
	fn as_expr_(&self) -> Option<&Self> {
		Some(self)
	}

	fn as_expr_mut_(&mut self) -> Option<&mut Self> {
		Some(self)
	}
}

pub trait TemplateLiteralExt<'a> {
	fn dbg_str(&self) -> Cow<'a, str>;
	fn is_literal(&self, ctx: &impl GlobalContext<'a>) -> bool;
}

impl<'a> TemplateLiteralExt<'a> for TemplateLiteral<'a> {
	fn dbg_str(&self) -> Cow<'a, str> {
		debug_assert_eq!(self.quasis.len(), self.expressions.len() + 1);
		if self.is_no_substitution_template() {
			return Cow::Borrowed(self.quasis[0].value.raw.as_str());
		}
		self.quasis
			.iter()
			.map(|q| q.value.raw)
			.join("${...}")
			.into()
	}

	fn is_literal(&self, ctx: &impl GlobalContext<'a>) -> bool {
		if self.is_no_substitution_template() {
			return true;
		}
		for expr in &self.expressions {
			if !expr.is_literal_value(false, ctx) {
				return false;
			}
		}
		true
	}
}

pub trait ArrayExpressionElementExt<'a>: ExpressionExt<'a> {
	fn as_array_expr_el_(&self) -> Option<&ArrayExpressionElement<'a>>;
	fn as_array_expr_el_mut_(
		&mut self,
	) -> Option<&mut ArrayExpressionElement<'a>>;
	fn as_spread(&self) -> Option<&SpreadElement<'a>> {
		match self.as_array_expr_el_()? {
			ArrayExpressionElement::SpreadElement(s) => Some(s.as_ref()),
			_ => None,
		}
	}
}

impl<'ast, T: ArrayExpressionElementExt<'ast>> ExpressionExt<'ast> for T {
	fn as_expr_(&self) -> Option<&Expression<'ast>> {
		self.as_array_expr_el_()?
			.as_expression()
	}

	fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>> {
		self.as_array_expr_el_mut_()?
			.as_expression_mut()
	}
}

impl<'a> ArrayExpressionElementExt<'a> for ArrayExpressionElement<'a> {
	fn as_array_expr_el_(&self) -> Option<&Self> {
		Some(self)
	}

	fn as_array_expr_el_mut_(&mut self) -> Option<&mut Self> {
		Some(self)
	}
}

pub trait BindingPatternExt {
	fn as_binding_identifier(&'_ self) -> Option<&'_ BindingIdentifier<'_>>;
}

impl BindingPatternExt for BindingPattern<'_> {
	fn as_binding_identifier(&'_ self) -> Option<&'_ BindingIdentifier<'_>> {
		match self {
			BindingPattern::BindingIdentifier(i) => Some(i.as_ref()),
			_ => None,
		}
	}
}

pub trait NumericLiteralExt {
	fn as_u32(&self) -> Option<u32>;
}

impl NumericLiteralExt for NumericLiteral<'_> {
	fn as_u32(&self) -> Option<u32> {
		let f: f64 = self.value;
		if f.is_finite()
			&& f.fract() == 0.
			&& f >= f64::from(u32::MIN)
			&& f <= f64::from(u32::MAX)
		{
			Some(f as u32)
		} else {
			None
		}
	}
}

#[derive(Debug, Copy, Clone, TryUnwrap)]
pub enum MemberExprAccessKind<'ast> {
	Static(&'ast IdentifierName<'ast>),
	Private(&'ast PrivateIdentifier<'ast>),
	Computed(&'ast Expression<'ast>),
}

impl GetSpan for MemberExprAccessKind<'_> {
	fn span(&self) -> Span {
		match self {
			MemberExprAccessKind::Static(i) => i.span(),
			MemberExprAccessKind::Private(i) => i.span(),
			MemberExprAccessKind::Computed(e) => e.span(),
		}
	}
}

impl<'ast> MemberExprAccessKind<'ast> {
	pub const fn from_member_expr(member_expr: MemberExprRef<'ast>) -> Self {
		match member_expr {
			MemberExprRef::Computed(ComputedMemberExpression {
				expression,
				..
			}) => Self::Computed(expression),
			MemberExprRef::Static(StaticMemberExpression {
				property, ..
			}) => Self::Static(property),
			MemberExprRef::Private(PrivateFieldExpression {
				field, ..
			}) => Self::Private(field),
		}
	}
}

#[derive(Copy, Clone, Debug, From)]
pub enum MemberExprRef<'ast> {
	/// [`ComputedMemberExpression`]
	Computed(&'ast ComputedMemberExpression<'ast>),
	/// [`StaticMemberExpression`]
	Static(&'ast StaticMemberExpression<'ast>),
	/// [`PrivateFieldExpression`]
	Private(&'ast PrivateFieldExpression<'ast>),
}

impl<'ast> MemberExprRef<'ast> {
	pub fn from_node(node: impl IntoAstKind<'ast>) -> Option<Self> {
		match node.into_ast_kind() {
			AstKind::ComputedMemberExpression(e) => Some(Self::Computed(e)),
			AstKind::StaticMemberExpression(e) => Some(Self::Static(e)),
			AstKind::PrivateFieldExpression(e) => Some(Self::Private(e)),
			_ => None,
		}
	}
	/// Gets the LHS of the member expression
	/// ### See
	/// [`ComputedMemberExpression::object`]
	///
	/// [`StaticMemberExpression::object`]
	///
	/// [`PrivateFieldExpression::object`]
	pub const fn left(self) -> &'ast Expression<'ast> {
		match self {
			MemberExprRef::Computed(ComputedMemberExpression {
				object,
				..
			})
			| MemberExprRef::Static(StaticMemberExpression {
				object, ..
			})
			| MemberExprRef::Private(PrivateFieldExpression {
				object, ..
			}) => object,
		}
	}

	/// get the rhs of the member expression
	pub const fn right(self) -> MemberExprAccessKind<'ast> {
		MemberExprAccessKind::from_member_expr(self)
	}
}
