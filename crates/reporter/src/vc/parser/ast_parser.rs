use crate::{
    util::Stage,
    vc::{
        Match, MatchLike, MatchRegex, Patch, Plugin, ReplaceLike, Replacement, Replacer,
        hash::hash_message_key,
        parser::exts::{
            ArrayExpressionElementExt as _, BindingPatternExt, ExpressionExt, ImportDeclarationExt,
            ModuleDeclarationExt, ObjectExpressionExt,
        },
    },
};
use anyhow::{Context, Result, bail};
use itertools::Itertools;
use memchr::memmem::Finder;
use oxc::{
    allocator::{Allocator, Box as OxcBox, Vec as OxcVec},
    ast::{
        AstBuilder, AstKind,
        ast::{
            Argument, ArrayExpressionElement, ArrowFunctionExpression, Expression,
            ImportDeclaration, ModuleDeclaration, ObjectExpression, Program, RegExpLiteral,
            SpreadElement, StringLiteral, TemplateLiteral,
        },
    },
    cfg::{BlockNodeId, ControlFlowGraph},
    minifier::PropertyReadSideEffects,
    parser::{Parser as OxcParser, ParserReturn, Token},
    semantic::{
        AstNode, NodeId, Semantic, SemanticBuilder,
        dot::{DebugDot, DebugDotContext},
    },
    span::{Atom, SourceType, Span}, syntax::module_record::ModuleRecord,
};
use oxc_ecmascript::{
    GlobalContext,
    constant_evaluation::{ConstantEvaluation, ConstantEvaluationCtx},
    side_effects::MayHaveSideEffectsContext,
};
use regress::Regex;
use std::{borrow::Cow, cell::Cell, fmt::Debug, sync::LazyLock};
use tracing::{debug, warn};

pub struct ParsedAst<'ast> {
    /// The parsed AST.
    ///
    /// Will be empty (e.g. no statements, directives, etc) if the parser panicked.
    ///
    /// ## Validity
    /// It is possible for the AST to be present and semantically invalid. This will happen if
    /// 1. The [`Parser`] encounters a recoverable syntax error
    /// 2. The logic for checking the violation is in the semantic analyzer
    ///
    /// To ensure a valid AST, check that [`errors`](ParserReturn::errors) is empty. Then, run
    /// semantic analysis with syntax error checking enabled.
    pub program: Program<'ast>,

    /// See <https://tc39.es/ecma262/#sec-abstract-module-records>
    pub module_record: ModuleRecord<'ast>,
    /// Lexed tokens in source order.
    ///
    /// Tokens are only collected when tokens are enabled in [`ParserConfig`].
    pub tokens: OxcVec<'ast, Token>,
}

pub trait AstParser<'ast> {
    fn parse(
        alloc: &'ast Allocator,
        source: &'ast str,
        source_type: SourceType,
    ) -> Result<(&'ast ParsedAst<'ast>, Semantic<'ast>)> {
        let parsed = OxcParser::new(alloc, source, source_type).parse();
        if parsed.panicked {
            bail!("Parser panicked while parsing source");
        }
        if !parsed.errors.is_empty() {
            bail!("Failed to parse source: {:#?}", parsed.errors);
        }
        let ast = alloc.alloc(ParsedAst {
            program: parsed.program,
            module_record: parsed.module_record,
            tokens: parsed.tokens,
        });
        let sema = SemanticBuilder::new()
            .with_cfg(true)
            .with_check_syntax_error(true)
            .build(&ast.program);
        if !sema.errors.is_empty() {
            bail!("Failed to perform semantic analysis on source: {:#?}", sema.errors);
        }
        Ok((ast, sema.semantic))
    }
    fn ast(&self) -> &'ast ParsedAst<'ast>;
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
        self.ast().program.body.iter().filter_map(|node| {
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
