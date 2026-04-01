use super::util::Ctx;
use ast_parser::{BindingPatternExt, ExpressionExt};
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
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct InlineConstantsPass<'ast> {
	marks: HashMap<ReferenceId, &'ast Expression<'ast>>,
}

fn should_inline<'ast, State>(
	node: &VariableDeclarator<'ast>,
	ctx: &mut TraverseCtx<'ast, State>,
) -> Option<SymbolId> {
	let ctx = Ctx(ctx);
	let sym_id = node
		.id
		.as_binding_identifier()?
		.symbol_id();
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

impl<'ast, State> Traverse<'ast, State> for InlineConstantsPass<'ast> {
	fn exit_variable_declaration(
		&mut self,
		node: &mut VariableDeclaration<'ast>,
		ctx: &mut TraverseCtx<'ast, State>,
	) {
		// export let foo = 1;
		// this does not cover `let foo = 1; export { foo };`, but it should be good enough for what we need it for
		// that case can probably be covered by iterating over the root
		// and looking for exports because they can only be at the top level
		if !node.kind.is_const()
			&& ctx
				.parent()
				.is_export_named_declaration()
		{
			return;
		}
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
		let Some(ref_id) = node
			.as_identifier()
			.map(IdentifierReference::reference_id)
		else {
			return;
		};
		if let Some(lit_expr) = self.marks.remove(&ref_id) {
			*node = lit_expr.clone_in(ctx.ast.allocator);
		}
	}
	fn exit_program(
		&mut self,
		_: &mut Program<'ast>,
		_: &mut TraverseCtx<'ast, State>,
	) {
		// TODO: fix this assert
		// debug_assert!(
		//     self.marks.is_empty(),
		//     "All marks should have been replaced by the end of the traversal. marks: {:#?}",
		//     self.marks
		// );
	}
}
#[cfg(test)]
mod tests {
	#![allow(clippy::needless_raw_string_hashes)]
	use super::*;
	use crate::test_pass;
	use insta::assert_snapshot;

	#[test]
	fn inlines_literal_constant_references() {
		let code = r#"
            const foo = 2;
            let bar = foo + 1;
            console.log(bar, foo);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        const foo = 2;
        let bar = 2 + 1;
        console.log(2 + 1, 2);
        ");
	}

	#[test]
	fn does_not_inline_mutated_binding() {
		let code = r#"
            let foo = 1;
            foo = 2;
            console.log(foo);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        let foo = 1;
        foo = 2;
        console.log(foo);
        ");
	}

	#[test]
	fn does_not_inline_non_literal_initializer() {
		let code = r#"
            const foo = Date.now();
            console.log(foo);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        const foo = Date.now();
        console.log(foo);
        ");
	}

	#[test]
	fn ignores_type_references() {
		let code = r#"
            const foo = { bar: "baz" };
            type Foo = typeof foo;
            console.log(foo);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        const foo = { bar: 'baz' };
        type Foo = typeof foo;
        console.log({ bar: 'baz' });
        ");
	}

	#[test]
	fn inlines_each_declarator_reference() {
		let code = r#"
            const foo = 1, bar = "x";
            console.log(foo, bar);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        const foo = 1, bar = 'x';
        console.log(1, 'x');
        ");
	}

	#[test]
	fn inlines_export_const() {
		let code = r#"
            export const foo = 1;
            console.log(foo);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        export const foo = 1;
        console.log(1);
        ");
	}

	#[test]
	fn does_not_inline_export_let() {
		let code = r#"
            export let foo = 1;
            console.log(foo);
        "#;
		let out = test_pass!(code, InlineConstantsPass::default());
		assert_snapshot!(out, @"
        export let foo = 1;
        console.log(foo);
        ");
	}
}
