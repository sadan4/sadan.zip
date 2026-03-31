mod inline_str;
mod util;
mod flatten_template;
mod fold_bin_exp;

use oxc::{
    allocator::Allocator,
    ast::ast::Program,
    semantic::{Scoping, Semantic, SemanticBuilder},
};
use oxc_traverse::{Traverse, traverse_mut};

pub use inline_str::InlineConstantLiteralsPass;
pub use flatten_template::FlattenTemplatePass;
pub use fold_bin_exp::FoldBinaryExpressionsPass;

pub struct PassManager<'ast> {
    program: &'ast mut Program<'ast>,
    scoping: Scoping,
    alloc: &'ast Allocator,
}

impl<'ast> PassManager<'ast> {
    pub fn new(
        alloc: &'ast Allocator,
        (program, scoping): (&'ast mut Program<'ast>, Scoping),
    ) -> Self {
        Self {
            program,
            scoping,
            alloc,
        }
    }
    pub fn run_pass(mut self, mut pass: impl Traverse<'ast, ()>) -> Self {
        let new_scoping = traverse_mut(&mut pass, self.alloc, self.program, self.scoping, ());
        self.scoping = new_scoping;
        self
    }
    pub fn finish(self) -> (&'ast Program<'ast>, Semantic<'ast>) {
        let prog = self.program;
        let sema = SemanticBuilder::new()
            .with_cfg(true)
            .with_check_syntax_error(true)
            .build(prog);
        assert!(sema.errors.is_empty(), "Passes created invalid AST: {:#?}", sema.errors);
        (prog, sema.semantic)
    }
}
