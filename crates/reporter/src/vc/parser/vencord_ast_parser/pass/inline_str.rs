use std::collections::HashMap;

use oxc::{
    allocator::CloneIn,
    ast::ast::{
        Expression, IdentifierReference, Program, VariableDeclaration,
        VariableDeclarator,
    },
    semantic::{ReferenceId, SymbolId},
};
use oxc_ecmascript::constant_evaluation::IsLiteralValue;
use oxc_traverse::{Traverse, TraverseCtx};

use crate::vc::parser::exts::{BindingPatternExt, ExpressionExt};
use crate::vc::parser::vencord_ast_parser::pass::util::Ctx;

#[derive(Default, Debug)]
pub struct InlineConstantLiteralsPass<'ast> {
    marks: HashMap<ReferenceId, &'ast Expression<'ast>>,
}

fn should_inline<'ast, State>(
    node: &VariableDeclarator<'ast>,
    ctx: &mut TraverseCtx<'ast, State>,
) -> Option<SymbolId> {
    let ctx = Ctx(ctx);
    let sym_id = node.id.as_binding_identifier()?.symbol_id();
    if ctx.scoping().symbol_is_mutated(sym_id) {
        return None;
    }
    // if !is_literal
    let is_literal = node
        .init
        .as_ref()
        .is_some_and(|init| init.is_literal_value(false, &ctx));
    if is_literal { Some(sym_id) } else { None }
}

impl<'ast, State> Traverse<'ast, State> for InlineConstantLiteralsPass<'ast> {
    fn exit_variable_declaration(
        &mut self,
        node: &mut VariableDeclaration<'ast>,
        ctx: &mut TraverseCtx<'ast, State>,
    ) {
        for i in (0..node.declarations.len()).rev() {
            if let Some(sym_id) = should_inline(&node.declarations[i], ctx) {
                // should never be None because we check it in should_inline
                let decl = ctx.ast.allocator.alloc(
                    node.declarations[i]
                        .init
                        .as_ref()
                        .unwrap()
                        .clone_in_with_semantic_ids(ctx.ast.allocator),
                );
                ctx.scoping()
                    .get_resolved_reference_ids(sym_id)
                    .iter()
                    .filter(|&&ref_id| {
                        let r = ctx.scoping().get_reference(ref_id);
                        !r.is_type()
                    })
                    .for_each(|&ref_id| {
                        self.marks.insert(ref_id, decl);
                    });
            }
        }
    }
    fn enter_expression(
        &mut self,
        node: &mut Expression<'ast>,
        ctx: &mut TraverseCtx<'ast, State>,
    ) {
        let Some(ref_id) = node.as_identifier().map(IdentifierReference::reference_id) else {
            return;
        };
        if let Some(lit_expr) = self.marks.remove(&ref_id) {
            *node = lit_expr.clone_in(ctx.ast.allocator);
        }
    }
    fn exit_program(&mut self, _: &mut Program<'ast>, _: &mut TraverseCtx<'ast, State>) {
        debug_assert!(
            self.marks.is_empty(),
            "All marks should have been replaced by the end of the traversal. marks: {:#?}",
            self.marks
        );
    }
}
