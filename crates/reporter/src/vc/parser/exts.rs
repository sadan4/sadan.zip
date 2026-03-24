use std::borrow::Cow;

use anyhow::{Result, bail};
use itertools::Itertools;
use oxc::allocator::Box as OxcBox;
use oxc::ast::ast::{
    ArrayExpression, ArrayExpressionElement, ArrowFunctionExpression, BindingIdentifier,
    BindingPattern, CallExpression, Expression, IdentifierReference, ImportDeclaration,
    ImportDeclarationSpecifier, ModuleDeclaration, ObjectExpression, ObjectProperty, PropertyKey,
    SpreadElement, StringLiteral, TemplateLiteral,
};
use oxc::semantic::SymbolId;

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
    fn default_or_namespace_var(&self) -> Option<SymbolId>;
    fn namespace_var(&self) -> Option<SymbolId>;
    fn default_var(&self) -> Option<SymbolId>;
    fn get_imported_var<'a>(&'a self, name: &str) -> Option<SymbolId>;
}

impl ImportDeclarationExt for ImportDeclaration<'_> {
    fn get_imported_var<'a>(&'a self, name: &str) -> Option<SymbolId> {
        let specifiers = self.specifiers.as_ref()?;
        for spec in specifiers {
            if let ImportDeclarationSpecifier::ImportSpecifier(i) = spec
                && i.imported.name() == name
            {
                return Some(i.local.symbol_id());
            }
        }
        None
    }
    fn default_var(&self) -> Option<SymbolId> {
        for spec in self.specifiers.as_ref()? {
            if let ImportDeclarationSpecifier::ImportDefaultSpecifier(i) = spec {
                return Some(i.local.symbol_id());
            }
        }
        None
    }
    fn namespace_var(&self) -> Option<SymbolId> {
        for spec in self.specifiers.as_ref()? {
            if let ImportDeclarationSpecifier::ImportNamespaceSpecifier(i) = spec {
                return Some(i.local.symbol_id());
            }
        }
        None
    }
    fn default_or_namespace_var(&self) -> Option<SymbolId> {
        for spec in self.specifiers.as_ref()? {
            match spec {
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    return Some(spec.local().symbol_id());
                }
                ImportDeclarationSpecifier::ImportSpecifier(_) => {}
            }
        }
        None
    }
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

pub trait ExpressionExt {
    fn as_identifier(&'_ self) -> Option<&'_ IdentifierReference<'_>>;
    fn as_string_literal(&'_ self) -> Option<&'_ StringLiteral<'_>>;
    fn as_object_expression(&'_ self) -> Option<&'_ ObjectExpression<'_>>;
    fn as_array_expression(&'_ self) -> Option<&'_ ArrayExpression<'_>>;
    fn as_call_expression(&'_ self) -> Option<&'_ CallExpression<'_>>;
    fn as_arrow_function_expression(&'_ self) -> Option<&'_ ArrowFunctionExpression<'_>>;
    fn dbg_name(&self) -> &'static str;
}

impl ExpressionExt for Expression<'_> {
    fn as_arrow_function_expression(&'_ self) -> Option<&'_ ArrowFunctionExpression<'_>> {
        match self {
            Expression::ArrowFunctionExpression(f) => Some(f.as_ref()),
            _ => None,
        }
    }
    fn as_string_literal(&'_ self) -> Option<&'_ StringLiteral<'_>> {
        match self {
            Expression::StringLiteral(s) => Some(s),
            _ => None,
        }
    }

    fn as_object_expression(&'_ self) -> Option<&'_ ObjectExpression<'_>> {
        match self {
            Expression::ObjectExpression(o) => Some(o.as_ref()),
            _ => None,
        }
    }

    fn dbg_name(&self) -> &'static str {
        match self {
            Self::BooleanLiteral(_) => "BooleanLiteral",
            Self::NullLiteral(_) => "NullLiteral",
            Self::NumericLiteral(_) => "NumericLiteral",
            Self::BigIntLiteral(_) => "BigIntLiteral",
            Self::RegExpLiteral(_) => "RegExpLiteral",
            Self::StringLiteral(_) => "StringLiteral",
            Self::TemplateLiteral(_) => "TemplateLiteral",
            Self::Identifier(_) => "Identifier",
            Self::MetaProperty(_) => "MetaProperty",
            Self::Super(_) => "Super",
            Self::ArrayExpression(_) => "ArrayExpression",
            Self::ArrowFunctionExpression(_) => "ArrowFunctionExpression",
            Self::AssignmentExpression(_) => "AssignmentExpression",
            Self::AwaitExpression(_) => "AwaitExpression",
            Self::BinaryExpression(_) => "BinaryExpression",
            Self::CallExpression(_) => "CallExpression",
            Self::ChainExpression(_) => "ChainExpression",
            Self::ClassExpression(_) => "ClassExpression",
            Self::ConditionalExpression(_) => "ConditionalExpression",
            Self::FunctionExpression(_) => "FunctionExpression",
            Self::ImportExpression(_) => "ImportExpression",
            Self::LogicalExpression(_) => "LogicalExpression",
            Self::NewExpression(_) => "NewExpression",
            Self::ObjectExpression(_) => "ObjectExpression",
            Self::ParenthesizedExpression(_) => "ParenthesizedExpression",
            Self::SequenceExpression(_) => "SequenceExpression",
            Self::TaggedTemplateExpression(_) => "TaggedTemplateExpression",
            Self::ThisExpression(_) => "ThisExpression",
            Self::UnaryExpression(_) => "UnaryExpression",
            Self::UpdateExpression(_) => "UpdateExpression",
            Self::YieldExpression(_) => "YieldExpression",
            Self::PrivateInExpression(_) => "PrivateInExpression",
            Self::JSXElement(_) => "JSXElement",
            Self::JSXFragment(_) => "JSXFragment",
            Self::TSAsExpression(_) => "TSAsExpression",
            Self::TSSatisfiesExpression(_) => "TSSatisfiesExpression",
            Self::TSTypeAssertion(_) => "TSTypeAssertion",
            Self::TSNonNullExpression(_) => "TSNonNullExpression",
            Self::TSInstantiationExpression(_) => "TSInstantiationExpression",
            Self::V8IntrinsicExpression(_) => "V8IntrinsicExpression",
            Self::ComputedMemberExpression(_) => "ComputedMemberExpression",
            Self::StaticMemberExpression(_) => "StaticMemberExpression",
            Self::PrivateFieldExpression(_) => "PrivateFieldExpression",
        }
    }

    fn as_array_expression(&'_ self) -> Option<&'_ ArrayExpression<'_>> {
        match self {
            Expression::ArrayExpression(a) => Some(a.as_ref()),
            _ => None,
        }
    }

    fn as_call_expression(&'_ self) -> Option<&'_ CallExpression<'_>> {
        match self {
            Expression::CallExpression(e) => Some(e.as_ref()),
            _ => None,
        }
    }
    fn as_identifier(&'_ self) -> Option<&'_ IdentifierReference<'_>> {
        match self {
            Self::Identifier(i) => Some(i.as_ref()),
            _ => None,
        }
    }
}

pub trait TemplateLiteralExt<'a> {
    fn dbg_str(&self) -> Cow<'a, str>;
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
