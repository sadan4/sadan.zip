use crate::vc::parser::exts::{ExpressionExt, TemplateLiteralExt};
use crate::vc::parser::vencord_ast_parser::pass::util::Ctx;
use oxc::ast::ast::Expression;
use oxc::ast::ast::TemplateElementValue;
use oxc_ecmascript::constant_evaluation::ConstantEvaluation;
use oxc_traverse::{Traverse, TraverseCtx};

pub struct FlattenTemplatePass;

impl<'ast, State> Traverse<'ast, State> for FlattenTemplatePass {
    fn enter_expression(
        &mut self,
        node: &mut Expression<'ast>,
        ctx: &mut TraverseCtx<'ast, State>,
    ) {
        let Some(template_node) = node.as_template_literal_mut() else {
            return;
        };

        if template_node.is_no_substitution_template() {
            debug_assert_eq!(template_node.quasis.len(), 1);
            debug_assert_eq!(template_node.expressions.len(), 0);
            let TemplateElementValue { raw, cooked } = template_node.quasis.remove(0).value;
            let cooked = cooked.unwrap();
            let lit = ctx
                .ast
                .alloc_string_literal(template_node.span, cooked, Some(raw));
            *node = Expression::StringLiteral(lit);
            return;
        }

        let ctx = Ctx(ctx);

        if template_node.is_literal(&ctx) {
            // we just checked that this has a literal value, so this should never be None
            let str_atom = ctx.eval_template(template_node);
            *node = ctx
                .ast
                .expression_string_literal(template_node.span, str_atom, None);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::needless_raw_string_hashes)]
    use super::*;
    use crate::test_pass;
    use insta::assert_snapshot;

    #[test]
    fn flattens_plain_template_literal() {
        let code = /* language=TypeScript */ r#"
            const msg = `hello world`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const msg = 'hello world';
            console.log(msg);
        ");
    }

    #[test]
    fn flattens_template_literal_in_expression_position() {
        let code = /* language=TypeScript */ r#"
            console.log(`hello`);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"console.log('hello');");
    }

    #[test]
    fn keeps_non_literal_template() {
        let code = /* language=TypeScript */ r#"
            const name = "world";
            const msg = `hello ${name}`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const name = 'world';
            const msg = `hello ${name}`;
            console.log(msg);
        ");
    }

    #[test]
    fn flattens_template_with_literal_expressions() {
        let code = /* language=TypeScript */ r#"
            const msg = `x${1 + 2}y${true}`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const msg = 'x3ytrue';
            console.log(msg);
        ");
    }
}
