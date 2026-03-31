use std::sync::Arc;
use itertools::Itertools;
use crate::vc::parser::exts::ModuleDeclarationExt;
use anyhow::{Result, bail};
use oxc::{
    allocator::{Allocator, Box as OxcBox},
    ast::ast::{ImportDeclaration, ModuleDeclaration, Program},
    parser::Parser as OxcParser,
    semantic::{AstNode, NodeId, Scoping, Semantic, SemanticBuilder},
    span::SourceType,
};

macro_rules! impl_parse {
    ($alloc:expr, $source:expr, $source_type:expr, $ast:ident, $sema:ident) => {
        let parsed = OxcParser::new($alloc, $source, $source_type).parse();
        if parsed.panicked {
            let dbg_src = Arc::new($source.to_string());
            let errs_with_src = parsed.errors.into_iter().map(move |err| err.with_source_code(dbg_src.clone())).collect_vec();
            bail!("OxcParser panicked while parsing source. errors: \n{:?}\n", errs_with_src);
        }
        if !parsed.errors.is_empty() {
            let dbg_src = Arc::new($source.to_string());
            let errs_with_src = parsed.errors.into_iter().map(move |err| err.with_source_code(dbg_src.clone())).collect_vec();
            bail!("Failed to parse source: \n{:#?}\n", errs_with_src);
        }
        let $ast: &'ast mut Program<'ast> = $alloc.alloc(parsed.program);
        let $sema = SemanticBuilder::new()
            .with_cfg(true)
            .with_check_syntax_error(true)
            .build($ast);
        if !$sema.errors.is_empty() {
            let dbg_src = Arc::new($source.to_string());
            let errs_with_src = $sema.errors.into_iter().map(move |err| err.with_source_code(dbg_src.clone())).collect_vec();
            bail!(
                "Failed to perform semantic analysis on source: \n{:#?}\n",
                errs_with_src
            );
        }
    };
}
pub trait AstParser<'ast> {
    // fn parse(
    //     alloc: &'ast Allocator,
    //     source: &'ast str,
    //     source_type: SourceType,
    // ) -> Result<(&'ast Program<'ast>, Semantic<'ast>)> {
    //     impl_parse!(alloc, source, source_type, ast, sema);
    //     Ok((ast, sema.semantic))
    // }

    fn parse_for_traverse(
        alloc: &'ast Allocator,
        source: &'ast str,
        source_type: SourceType,
    ) -> Result<(&'ast mut Program<'ast>, Scoping)> {
        impl_parse!(alloc, source, source_type, ast, sema);
        let scoping = sema.semantic.into_scoping();
        Ok((ast, scoping))
    }
    fn prog(&self) -> &'ast Program<'ast>;
    fn sema(&self) -> &Semantic<'ast>;
    // /// node from id
    // fn n<'a: 'ast>(&'a self, node_id: NodeId) -> &'ast AstNode<'ast> {
    //     self.sema().nodes().get_node(node_id)
    // }
    /// Parent of node
    fn p<'a: 'ast>(&'a self, node_id: NodeId) -> &'ast AstNode<'ast> {
        self.sema().nodes().parent_node(node_id)
    }
    // fn cfg_id(&self, node_id: NodeId) -> BlockNodeId {
    //     self.sema().nodes().cfg_id(node_id)
    // }
    // fn cfg<'a: 'ast>(&'a self) -> &'ast ControlFlowGraph {
    //     // we always parse with the cfg
    //     self.sema().cfg().unwrap()
    // }
    // fn dbg_cfg<'a: 'ast>(&'a self, node_id: NodeId) -> String {
    //     let cfg_id = self.cfg_id(node_id);
    //     let cfg = self.cfg();
    //     let block = cfg.basic_block(cfg_id);
    //     let ctx = DebugDotContext::new(self.sema().nodes(), true);
    //     block.debug_dot(ctx)
    // }
}

pub trait ESModuleParser<'ast>: AstParser<'ast> {
    fn import_statements<'a: 'ast>(
        &'a self,
    ) -> impl Iterator<Item = &'ast ImportDeclaration<'ast>> {
        self.prog().body.iter().filter_map(|node| {
            node.as_module_declaration()
                .and_then(ModuleDeclaration::as_import_declaration)
                .map(OxcBox::as_ref)
        })
    }
    fn find_import_by_name<'a: 'ast, 'b>(
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
}
