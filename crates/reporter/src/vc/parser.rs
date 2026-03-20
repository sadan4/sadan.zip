use std::path::Path;

use crate::vc::Patch;
use anyhow::{Context, Result, bail};
use oxc::{
    allocator::Allocator,
    ast::ast::ModuleDeclaration,
    parser::{Parser, ParserReturn},
    semantic::{Semantic, SemanticBuilder},
    span::SourceType,
};
use tokio::fs;

async fn parse_patches(plugin_entry: &Path) -> Result<Vec<Patch>> {
    let allocator = Allocator::new();
    let content = fs::read_to_string(plugin_entry).await?;
    let source_type = SourceType::from_path(plugin_entry)
        .context("Failed to parse source type for plugin entry")?;

    let ast = Parser::new(&allocator, &content, source_type).parse();
    let sema = verify_and_make_sema(&ast)?;

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

struct Parser<'ast> {
    allocator: &'ast Allocator,
    ast: ParserReturn<'ast>,
    sema: Semantic<'ast>,
    c: ParserCache<'ast>,
}

struct ParserCache<'ast> {}

impl Parser<'_> {
    fn find_import_by_name(&self, from: &str) -> Option<()> {
        // Imports can only be at the top level
        for node in self.ast.program.body.iter().filter_map(|node| {
            node.as_module_declaration()
                .and_then(|m_decl| match m_decl {
                    ModuleDeclaration::ImportDeclaration(i) => Some(i),
                    _ => None,
                })
        }) {
            dbg!(node);
        }
        None
    }
}
