mod exts;
use std::{cell::Cell, fs, path::Path};

use crate::vc::{
    Patch,
    parser::exts::{
        ExpressionExt, ImportDeclarationExt, ModuleDeclarationExt, ObjectExpressionExt,
    },
};
use anyhow::{Context, Result, bail};
use oxc::{
    allocator::{Allocator, Box},
    ast::{
        AstKind,
        ast::{Argument, ImportDeclaration, ModuleDeclaration, ObjectExpression},
    },
    parser::{Parser as OxcParser, ParserReturn},
    semantic::{AstNode, NodeId, Semantic, SemanticBuilder},
    span::SourceType,
};
use tracing::info;

pub fn parse_patches(allocator: &Allocator, plugin_entry: &Path) -> Result<Vec<Patch>> {
    let content = fs::read_to_string(plugin_entry)?;
    let source_type = SourceType::from_path(plugin_entry)
        .context("Failed to parse source type for plugin entry")?;

    let ast = OxcParser::new(allocator, &content, source_type).parse();
    let sema = verify_and_make_sema(&ast)?;

    let parser = Parser {
        allocator,
        ast: &ast,
        sema,
        c: ParserCache::default(),
    };

    info!("plugin name: {:?}", parser.plugin_name());

    Ok(vec![])
}

fn verify_and_make_sema<'a>(ast: &'a ParserReturn<'a>) -> Result<Semantic<'a>> {
    if !ast.errors.is_empty() {
        // Do we need to clone here?
        Err(ast.errors[0].clone()).context("Failed to parse plugin")?;
    }
    if ast.panicked {
        bail!("Parser panicked while parsing plugin");
    }
    // run semantic analysis
    let mut sema_result = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(&ast.program);

    if !sema_result.errors.is_empty() {
        Err(sema_result.errors.swap_remove(0)).context("Failed to validate plugin AST")?;
    }

    Ok(sema_result.semantic)
}

pub struct Parser<'ast> {
    allocator: &'ast Allocator,
    ast: &'ast ParserReturn<'ast>,
    sema: Semantic<'ast>,
    c: ParserCache<'ast>,
}

#[derive(Default)]
struct ParserCache<'ast> {
    define_plugin: Cell<Option<Option<&'ast ObjectExpression<'ast>>>>,
}

const DEFINE_PLUGIN_IMPORT_SOURCE: &str = "@utils/types";

impl<'ast> Parser<'ast> {
    /// node from id
    pub fn n<'a: 'ast>(&'a self, node_id: NodeId) -> &'ast AstNode<'ast> {
        self.sema.nodes().get_node(node_id)
    }
    /// Parent of node
    pub fn p<'a: 'ast>(&'a self, node_id: NodeId) -> &'ast AstNode<'ast> {
        self.sema.nodes().parent_node(node_id)
    }
    pub fn import_statements(&self) -> impl Iterator<Item = &ImportDeclaration> {
        self.ast.program.body.iter().filter_map(|node| {
            node.as_module_declaration()
                .and_then(ModuleDeclaration::as_import_declaration)
                .map(Box::as_ref)
        })
    }
    pub fn find_import_by_name<'a: 'ast, 'b>(
        &'a self,
        from: &'b str,
    ) -> Option<&'ast ImportDeclaration<'ast>> {
        let pred = |import: &&ImportDeclaration| import.source.value == from;
        debug_assert!(
            self.import_statements().filter(pred).count() <= 1,
            "Found multiple import statements with the same source"
        );
        // Imports can only be at the top level
        self.import_statements().find(pred)
    }
    pub fn define_plugin_<'a: 'ast>(&'a self) -> Option<&'ast ObjectExpression<'ast>> {
        let define_plugin = self
            .find_import_by_name(DEFINE_PLUGIN_IMPORT_SOURCE)?
            .default_var()?;
        for var_use in self.sema.symbol_references(define_plugin) {
            let AstKind::CallExpression(call) = self.p(var_use.node_id()).kind() else {
                continue;
            };
            if call.arguments.len() != 1 {
                continue;
            }
            let Argument::ObjectExpression(obj) = &call.arguments[0] else {
                continue;
            };
            return Some(obj.as_ref());
        }
        None
    }
    pub fn define_plugin<'a: 'ast>(&'a self) -> Option<&'ast ObjectExpression<'ast>> {
        if let Some(s) = self.c.define_plugin.get() {
            return s;
        }
        let ret = self.define_plugin_();

        self.c.define_plugin.set(Some(ret));

        ret
    }
    pub fn plugin_name<'a: 'ast>(&'a self) -> Option<String> {
        Some(
            self.define_plugin()?
                .get_property("name")?
                .value
                .as_string_literal()?
                .value
                .to_string(),
        )
    }
}
