use super::util::Ctx;
use ast_parser::exts::ExpressionExt as _;
use oxc::{
	allocator::TakeIn,
	ast::ast::{Expression, TemplateElementValue},
};
use oxc_traverse::Traverse;

pub struct EvalStringRawPass;

impl<'ast> Traverse<'ast, ()> for EvalStringRawPass {
	fn enter_expression(
		&mut self,
		node: &mut oxc::ast::ast::Expression<'ast>,
		ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
	) {
		let Some(tagged_template) = node.as_tagged_template_mut() else {
			return;
		};
		let Some(tag_expr) = tagged_template
			.tag
			.get_inner_expression()
			.as_member_expression()
		else {
			return;
		};
		if !tag_expr.is_specific_member_access("String", "raw") {
			return;
		}
		let ctx = Ctx(ctx);
		let mut raw_template = tagged_template.quasi.take_in_box(&ctx);
		let iter = raw_template.quasis.iter_mut();
		for elem in iter {
			let old_raw = elem.value.raw;
			*elem = ctx.ast.template_element(
				elem.span,
				TemplateElementValue {
					raw: old_raw,
					cooked: Some(old_raw),
				},
				elem.tail,
				true,
			);
		}
		*node = Expression::TemplateLiteral(raw_template);
	}
}

#[cfg(test)]
mod tests {
	#![allow(clippy::needless_raw_string_hashes)]
	use insta::assert_snapshot;

	use crate::test_pass;

	use super::*;
	#[test]
	fn test_inline_string_raw() {
		let code = /* language=Typescript */ r#"
            const foo = String.raw`/^(?:([a-z0-9_+\-.#]+?)\n)?\n*([^\n][^]*?)\n*`;
        "#;
		let out = test_pass!(code, EvalStringRawPass);
		assert_snapshot!(out, /* language=Typescript */ @r"const foo = `/^(?:([a-z0-9_+\\-.#]+?)\\n)?\\n*([^\\n][^]*?)\\n*`;");
	}
}
