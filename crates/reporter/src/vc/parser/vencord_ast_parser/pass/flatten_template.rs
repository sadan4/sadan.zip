use crate::vc::parser::exts::{ExpressionExt, TemplateLiteralExt};
use crate::vc::parser::vencord_ast_parser::pass::util::Ctx;
use oxc::{ast::ast::Expression, span::Atom};
use oxc_ecmascript::ToJsString;
use oxc_traverse::{Traverse, TraverseCtx};

pub struct FlattenTemplatePass;

impl<'ast, State> Traverse<'ast, State> for FlattenTemplatePass {
    fn enter_expression(
        &mut self,
        node: &mut Expression<'ast>,
        ctx: &mut TraverseCtx<'ast, State>,
    ) {
        let Some(template_node) = node.as_template_literal() else {
            return;
        };

        let ctx = Ctx(ctx);

        if !template_node.is_literal(&ctx) {
            return;
        }

        // we just checked that this has a literal value, so this should never be None
        let str_val = template_node.to_js_string(&ctx).unwrap();
        let str_atom = Atom::from_cow_in(&str_val, ctx.ast.allocator);
        *node = ctx.ast.expression_string_literal(template_node.span, str_atom, None);
    }
}
