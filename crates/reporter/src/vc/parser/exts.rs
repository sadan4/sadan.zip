use oxc::allocator::Box as OxcBox;
use oxc::ast::ast::{
    Expression, ImportDeclaration, ImportDeclarationSpecifier, ModuleDeclaration, ObjectExpression, ObjectProperty, PropertyKey, StringLiteral
};
use oxc::semantic::SymbolId;

pub trait ModuleDeclarationExt {
    fn as_import_declaration(&self) -> Option<&OxcBox<ImportDeclaration>>;
}

impl ModuleDeclarationExt for ModuleDeclaration<'_> {
    fn as_import_declaration(&self) -> Option<&OxcBox<ImportDeclaration>> {
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
                _ => {}
            }
        }
        None
    }
}

pub trait ObjectExpressionExt {
    fn get_property<'a>(&'a self, name: &str) -> Option<&ObjectProperty>;
}

impl ObjectExpressionExt for ObjectExpression<'_> {
    fn get_property<'a>(&'a self, name: &str) -> Option<&ObjectProperty> {
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
    fn as_string_literal(&self) -> Option<&StringLiteral>;
}

impl ExpressionExt for Expression<'_> {
    fn as_string_literal(&self) -> Option<&StringLiteral> {
        match self {
            Expression::StringLiteral(s) => Some(s),
            _ => None,
        }
    }
}