use anyhow::{Result, bail};
use itertools::Itertools as _;
use oxc::{
	allocator::Box as OxcBox,
	ast::ast::{
		Argument,
		ArrayExpression,
		ArrayExpressionElement,
		ArrowFunctionExpression,
		BinaryExpression,
		BindingIdentifier,
		BindingPattern,
		CallExpression,
		ComputedMemberExpression,
		Expression,
		ExpressionStatement,
		Function,
		IdentifierName,
		IdentifierReference,
		ImportDeclaration,
		ImportDeclarationSpecifier,
		MemberExpression,
		ModuleDeclaration,
		NumericLiteral,
		ObjectExpression,
		ObjectProperty,
		PrivateFieldExpression,
		PrivateIdentifier,
		PropertyKey,
		SpreadElement,
		Statement,
		StaticMemberExpression,
		StringLiteral,
		TaggedTemplateExpression,
		TemplateLiteral,
	},
	semantic::SymbolId,
	span::Atom,
};
use oxc_ecmascript::{GlobalContext, constant_evaluation::IsLiteralValue};
use std::borrow::Cow;

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
	/// returns Err(e) if present but not a bool
	fn parse_bool_flag(&self, key: &str) -> Result<bool> {
		match self.get_property(key) {
			Some(prop) => match &prop.value {
				Expression::BooleanLiteral(b) => Ok(b.value),
				_ => bail!("not a boolean literal"),
			},
			None => Ok(false),
		}
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

impl<'ast, T: ExpressionExt<'ast>> MemberExpressionExt<'ast> for T {
	fn as_member_expr_(&self) -> Option<&MemberExpression<'ast>> {
		self.as_expr_()?.as_member_expression()
	}

	fn as_member_expr_mut_(&mut self) -> Option<&mut MemberExpression<'ast>> {
		self.as_expr_mut_()?
			.as_member_expression_mut()
	}
}

pub trait ExpressionExt<'ast> {
	fn as_expr_(&self) -> Option<&Expression<'ast>>;
	fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>>;

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

	fn as_string_literal_like(&self) -> Option<Atom<'ast>> {
		match self.as_expr_()? {
			Expression::StringLiteral(s) => s.raw,
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
			Expression::MetaProperty(_) => "MetaProperty",
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
	fn as_array_expr_el_mut_(&mut self) -> Option<&mut ArrayExpressionElement<'a>>;
	fn as_spread(&self) -> Option<&SpreadElement<'a>> {
		match self.as_array_expr_el_()? {
			ArrayExpressionElement::SpreadElement(s) => Some(s.as_ref()),
			_ => None,
		}
	}
}

impl<'ast, T: ArrayExpressionElementExt<'ast>> ExpressionExt<'ast> for T {
	fn as_expr_(&self) -> Option<&Expression<'ast>> {
		self.as_array_expr_el_()?.as_expression()
	}

	fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>> {
		self.as_array_expr_el_mut_()?.as_expression_mut()
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
