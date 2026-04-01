use std::borrow::Cow;

use anyhow::{Result, bail};
use itertools::Itertools as _;
use oxc::allocator::Box as OxcBox;
use oxc::ast::ast::{
    ArrayExpression, ArrayExpressionElement, ArrowFunctionExpression, BinaryExpression, BindingIdentifier, BindingPattern, CallExpression, Expression, IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, ModuleDeclaration, ObjectExpression, ObjectProperty, PropertyKey, SpreadElement, StringLiteral, TaggedTemplateExpression, TemplateLiteral
};
use oxc::semantic::SymbolId;
use oxc_ecmascript::GlobalContext;
use oxc_ecmascript::constant_evaluation::IsLiteralValue;

pub trait ModuleDeclarationExt {
    fn as_import_declaration(&'_ self) -> Option<&'_ OxcBox<'_, ImportDeclaration<'_>>>;
}

impl ModuleDeclarationExt for ModuleDeclaration<'_> {
    fn as_import_declaration(&'_ self) -> Option<&'_ OxcBox<'_, ImportDeclaration<'_>>> {
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
            if let ImportDeclarationSpecifier::ImportDefaultSpecifier(i) = spec {
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
    fn get_property<'a>(&'a self, name: &str) -> Option<&'a ObjectProperty<'a>>;
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
    fn get_property<'a>(&'a self, name: &str) -> Option<&'a ObjectProperty<'a>> {
        for prop in &self.properties {
            let Some(prop) = prop.as_property() else {
                continue;
            };
            match &prop.key {
                PropertyKey::StaticIdentifier(i) if i.name == name => {
                    return Some(prop);
                }
                PropertyKey::NumericLiteral(num) if num.value.to_string() == name => {
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

pub trait ExpressionExt<'ast> {
    fn as_expr_(&self) -> Option<&Expression<'ast>>;
    fn as_expr_mut_(&mut self) -> Option<&mut Expression<'ast>>;

    fn as_arrow_function_expression(&self) -> Option<&ArrowFunctionExpression<'ast>> {
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
            Expression::TaggedTemplateExpression(_) => "TaggedTemplateExpression",
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
            Expression::TSInstantiationExpression(_) => "TSInstantiationExpression",
            Expression::V8IntrinsicExpression(_) => "V8IntrinsicExpression",
            Expression::ComputedMemberExpression(_) => "ComputedMemberExpression",
            Expression::StaticMemberExpression(_) => "StaticMemberExpression",
            Expression::PrivateFieldExpression(_) => "PrivateFieldExpression",
        }
    }

    fn as_array_expression(&self) -> Option<&ArrayExpression<'ast>> {
        match self.as_expr_()? {
            Expression::ArrayExpression(a) => Some(a.as_ref()),
            _ => None,
        }
    }

    fn as_call_expression(&self) -> Option<&CallExpression<'ast>> {
        match self.as_expr_()? {
            Expression::CallExpression(e) => Some(e.as_ref()),
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
    fn as_template_literal_mut(&mut self) -> Option<&mut TemplateLiteral<'ast>> {
        match self.as_expr_mut_()? {
            Expression::TemplateLiteral(i) => Some(i.as_mut()),
            _ => None,
        }
    }
    fn as_tagged_template_mut(&mut self) -> Option<&mut TaggedTemplateExpression<'ast>> {
        match self.as_expr_mut_()? {
            Expression::TaggedTemplateExpression(e) => Some(e.as_mut()),
            _ => None,
        }
    }
    // fn as_binary_expression(&self) -> Option<&BinaryExpression<'ast>> {
    //     match self.as_expr_()? {
    //         Expression::BinaryExpression(e) => Some(e.as_ref()),
    //         _ => None,
    //     }
    // }
    fn as_binary_expression_mut(&mut self) -> Option<&mut BinaryExpression<'ast>> {
        match self.as_expr_mut_()? {
            Expression::BinaryExpression(e) => Some(e.as_mut()),
            _ => None,
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

pub trait ArrayExpressionElementExt<'a> {
    fn as_spread<'b: 'a>(&'b self) -> Option<&'a SpreadElement<'a>>;
    fn dbg_name(&self) -> &'static str;
}

impl<'a> ArrayExpressionElementExt<'a> for ArrayExpressionElement<'a> {
    fn as_spread<'b: 'a>(&'b self) -> Option<&'a SpreadElement<'a>> {
        match self {
            ArrayExpressionElement::SpreadElement(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    fn dbg_name(&self) -> &'static str {
        self.as_expression().map_or_else(
            || match self {
                Self::SpreadElement(_) => "SpreadElement",
                Self::Elision(_) => "Elision",
                _ => unreachable!(),
            },
            Expression::dbg_name,
        )
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
