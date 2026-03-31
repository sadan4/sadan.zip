use crate::vc::parser::exts::{ExpressionExt, TemplateLiteralExt};
use crate::vc::parser::vencord_ast_parser::pass::util::Ctx;
use oxc::ast::ast::Expression;
use oxc::ast::ast::TemplateElementValue;
use oxc::span::GetSpan as _;
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
        // try to inline as many literal expressions as we can
        debug_assert_eq!(
            template_node.quasis.len(),
            template_node.expressions.len() + 1
        );
        for i in (0..template_node.expressions.len()).rev() {
            let expr = &template_node.expressions[i];
            if let Some(str_val) = expr.evaluate_value_to_string(&ctx) {
                let right = template_node.quasis.remove(i + 1);
                let left = &mut template_node.quasis[i];
                let expr_span = template_node.expressions.remove(i).span();
                let new_raw = ctx
                    .a()
                    .alloc_concat_strs_array([
                        left.value.raw.as_str(),
                        &str_val,
                        right.value.raw.as_str(),
                    ])
                    .into();
                let new_cooked = ctx
                    .a()
                    .alloc_concat_strs_array([
                        left.value.cooked.unwrap().as_str(),
                        &str_val,
                        right.value.cooked.unwrap().as_str(),
                    ])
                    .into();
                left.lone_surrogates |= right.lone_surrogates;
                left.tail = right.tail;
                left.span = left.span.merge(right.span).merge(expr_span);
                left.value = TemplateElementValue {
                    raw: new_raw,
                    cooked: Some(new_cooked),
                };
            }
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

    #[test]
    fn partially_flattens_template_with_some_literal_expressions() {
        let code = /* language=TypeScript */ r#"
            const x = "dynamic";
            const msg = `start${123}middle${x}end`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const x = 'dynamic';
            const msg = `start123middle${x}end`;
            console.log(msg);
        ");
    }

    #[test]
    fn partially_flattens_template_with_multiple_literal_expressions() {
        let code = /* language=TypeScript */ r#"
            const x = "dynamic";
            const msg = `a${1}b${2}c${x}d${3}e`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const x = 'dynamic';
            const msg = `a1b2c${x}d3e`;
            console.log(msg);
        ");
    }

    #[test]
    fn partially_flattens_template_with_boolean_and_number_literals() {
        let code = /* language=TypeScript */ r#"
            const msg = `prefix${false}${name}${42}suffix`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const msg = `prefixfalse${name}42suffix`;
            console.log(msg);
        ");
    }

    #[test]
    fn partially_flattens_with_consecutive_literals() {
        let code = /* language=TypeScript */ r#"
            const x = "var";
            const msg = `${1}${2}${3}${x}${4}${5}`;
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        // Processes right to left: merges ${4}${5}, then stops at ${x}
        // Note: 'var' literal stays because it goes through plain template flatten first
        assert_snapshot!(out, /* language=TypeScript */ @"
            const x = 'var';
            const msg = `123${x}45`;
        ");
    }

    #[test]
    fn handles_template_with_only_non_literal_expressions() {
        let code = /* language=TypeScript */ r#"
            const msg = `${foo}${bar}${baz}`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const msg = `${foo}${bar}${baz}`;
            console.log(msg);
        ");
    }

    #[test]
    fn handles_bigints_less_than_2_32() {
        let code = /* language=TypeScript */ r#"
            const msg = `value: ${123n}`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const msg = 'value: 123';
            console.log(msg);
        ");
    }

    #[test]
    fn handles_bigints_greater_than_2_32() {
        let code = /* language=TypeScript */ r#"
            const msg = `large: ${9007199254740991n}`;
            console.log(msg);
        "#;
        let out = test_pass!(code, FlattenTemplatePass);
        assert_snapshot!(out, /* language=TypeScript */ @"
            const msg = 'large: 9007199254740991';
            console.log(msg);
        ");
    }
}
