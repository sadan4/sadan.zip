use crate::pass::util::{Ctx, empty_template_element_value};
use ast_parser::exts::ExpressionExt as _;
use oxc::{
	ast::ast::{BinaryOperator, Expression, TemplateElementValue},
	span::{Atom, Span},
};
use oxc_ecmascript::constant_evaluation::{
	ConstantValue,
	binary_operation_evaluate_value,
};
use oxc_traverse::Traverse;
use std::mem;
use tracing::warn;

pub struct FoldBinaryExpressionsPass;

/// Fold `BigInt` left shift operations (e.g., 1n << 20n)
fn try_fold_bigint_shift<'ast>(
	left: &Expression<'ast>,
	right: &Expression<'ast>,
) -> Option<ConstantValue<'ast>> {
	// Extract BigInt literals from both sides
	let Expression::BigIntLiteral(left_lit) = left else {
		return None;
	};
	let Expression::BigIntLiteral(right_lit) = right else {
		return None;
	};

	// Parse the raw string representation to get the BigInt value
	// The raw value includes the 'n' suffix (e.g., "123n"), so we need to strip it
	let left_str = left_lit.raw.as_ref()?.as_str();
	let right_str = right_lit.raw.as_ref()?.as_str();

	let left_value = left_str
		.strip_suffix('n')
		.unwrap_or(left_str);
	let right_value = right_str
		.strip_suffix('n')
		.unwrap_or(right_str);

	let left_bigint: num_bigint::BigInt = left_value.parse().ok()?;
	let right_bigint: num_bigint::BigInt = right_value.parse().ok()?;

	// Convert right operand to u64 for shift amount
	// BigInt shift operations require the shift amount to fit in u64
	let shift = right_bigint.to_u64_digits();
	if shift.1.len() != 1 {
		// Shift amount is too large or negative
		return None;
	}
	let shift_bits = shift.1[0];

	// Perform the left shift
	let result = left_bigint << shift_bits;
	Some(ConstantValue::BigInt(result))
}

// TODO: refactor
#[expect(clippy::too_many_lines)]
fn fold_template_literals<'ast, State>(
	left: &mut Expression<'ast>,
	right: &mut Expression<'ast>,
	span: Span,
	ctx: &Ctx<'_, 'ast, State>,
) -> Option<Expression<'ast>> {
	if let Some(left) = left.as_string_literal() {
		// prepare `left`
		let left_val = left.value.as_str();
		// we should only ever be here if left or right is a template literal
		// and left is a string, so right must be a template literal
		let right = right.as_template_literal_mut().unwrap();
		let q1 = &mut right.quasis[0];
		let new_q1_raw = ctx
			.ast
			.allocator
			.alloc_concat_strs_array([left_val, q1.value.raw.as_str()]);
		let new_q1_val = ctx
			.ast
			.allocator
			.alloc_concat_strs_array([
				left_val,
				q1.value.cooked.unwrap().as_str(),
			]);
		let new_q1 = ctx.ast.template_element(
			q1.span,
			TemplateElementValue {
				raw: Atom::from(new_q1_raw),
				cooked: Some(Atom::from(new_q1_val)),
			},
			q1.tail,
			false,
		);
		right.quasis[0] = new_q1;
		right.span = span;
		let ret = mem::replace(right, ctx.dummy());
		let ret = ctx.alloc(ret);
		let ret = Expression::TemplateLiteral(ret);
		Some(ret)
	} else if let Some(right) = right.as_string_literal() {
		// prepare `right`
		let right_val = right.value.as_str();
		let left = left.as_template_literal_mut().unwrap();
		let last_idx = left.quasis.len() - 1;
		let q = &mut left.quasis[last_idx];
		let new_q_raw = ctx
			.ast
			.allocator
			.alloc_concat_strs_array([q.value.raw.as_str(), right_val]);
		let new_q_val = ctx
			.ast
			.allocator
			.alloc_concat_strs_array([
				q.value.cooked.unwrap().as_str(),
				right_val,
			]);
		debug_assert!(
			q.tail,
			"last element of template literal should have tail=true"
		);
		let new_q = ctx.ast.template_element(
			q.span,
			TemplateElementValue {
				raw: Atom::from(new_q_raw),
				cooked: Some(Atom::from(new_q_val)),
			},
			q.tail,
			false,
		);
		left.quasis[last_idx] = new_q;
		left.span = span;
		let ret = mem::replace(left, ctx.dummy());
		let ret = ctx.alloc(ret);
		let ret = Expression::TemplateLiteral(ret);
		Some(ret)
	} else if let Some(left) = left.as_identifier_mut() {
		let right = right.as_template_literal_mut().unwrap();
		let mut right = ctx.take(right);
		let left = ctx.take(left);
		let new_q = ctx.ast.template_element(
			Span::new(right.span.start, right.span.start),
			empty_template_element_value(),
			false,
			false, // we are inserting an empty element, no need to escape
		);
		right
			.expressions
			.insert(0, Expression::Identifier(ctx.alloc(left)));
		right.quasis.insert(0, new_q);
		right.span = span;
		let ret = ctx.alloc(right);
		let ret = Expression::TemplateLiteral(ret);
		Some(ret)
	} else if let Some(right) = right.as_identifier_mut() {
		let left = left.as_template_literal_mut().unwrap();
		let mut left = ctx.take(left);
		let right = ctx.take(right);
		left.quasis.last_mut().unwrap().tail = false;
		let new_q = ctx.ast.template_element(
			Span::new(left.span.end, left.span.end),
			empty_template_element_value(),
			true,
			false, // we are inserting an empty element, no need to escape
		);
		left.expressions
			.push(Expression::Identifier(ctx.alloc(right)));
		left.quasis.push(new_q);
		left.span = span;
		let ret = ctx.alloc(left);
		let ret = Expression::TemplateLiteral(ret);
		Some(ret)
	} else if let Some(right) = right.as_template_literal_mut()
		&& let Some(left) = left.as_template_literal_mut()
	{
		let left_last_idx = left.quasis.len() - 1;
		let left_joiner = &mut left.quasis[left_last_idx];
		let right_joiner = &mut right.quasis[0];
		let joiner_raw = ctx
			.ast
			.allocator
			.alloc_concat_strs_array([
				left_joiner.value.raw.as_str(),
				right_joiner.value.raw.as_str(),
			]);
		let joiner_cooked = ctx
			.ast
			.allocator
			.alloc_concat_strs_array([
				left_joiner
					.value
					.cooked
					.unwrap()
					.as_str(),
				right_joiner
					.value
					.cooked
					.unwrap()
					.as_str(),
			]);
		let joiner = ctx.ast.template_element(
			left_joiner.span,
			TemplateElementValue {
				raw: Atom::from(joiner_raw),
				cooked: Some(Atom::from(joiner_cooked)),
			},
			right_joiner.tail,
			false,
		);
		*left_joiner = joiner;
		left.quasis
			.extend(right.quasis.drain(..).skip(1));
		left.expressions
			.extend(right.expressions.drain(..));
		left.span = span;
		let ret = mem::replace(left, ctx.dummy());
		let ret = ctx.alloc(ret);
		let ret = Expression::TemplateLiteral(ret);
		Some(ret)
	} else {
		warn!(
			"unhandled bin exp fold case: left:{}, right:{}",
			left.dbg_name(),
			right.dbg_name(),
		);
		None
	}
}

impl<'ast, State> Traverse<'ast, State> for FoldBinaryExpressionsPass {
	fn exit_expression(
		&mut self,
		expr_node: &mut Expression<'ast>,
		ctx: &mut oxc_traverse::TraverseCtx<'ast, State>,
	) {
		let Some(node) = expr_node.as_binary_expression_mut() else {
			return;
		};
		let ctx = Ctx(ctx);
		let left = &mut node.left;
		let right = &mut node.right;
		let op = node.operator;

		// Try custom BigInt shift folding first
		if op == BinaryOperator::ShiftLeft
			&& let Some(val) = try_fold_bigint_shift(left, right)
		{
			*expr_node = ctx.node_from_constant_value(val, node.span);
			return;
		}

		if let Some(val) =
			binary_operation_evaluate_value(op, left, right, &ctx)
		{
			*expr_node = ctx.node_from_constant_value(val, node.span);
			return;
		}
		if op != BinaryOperator::Addition {
			return;
		}
		if (left.is_template_literal() || right.is_template_literal())
			&& let Some(new_expr) =
				fold_template_literals(left, right, node.span, &ctx)
		{
			*expr_node = new_expr;
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
	fn folds_constant_binary_expression() {
		let code = /* language=TypeScript */ r#"
            const value = 1 + 2;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 3;
        console.log(value);
        ");
	}

	#[test]
	fn folds_string_plus_template_literal() {
		let code = /* language=TypeScript */ r#"
            const left = "x";
            const mid = 1;
            const out = left + `${mid}y`;
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const left = 'x';
        const mid = 1;
        const out = `${left}${mid}y`;
        ");
	}

	#[test]
	fn folds_template_literal_plus_string() {
		let code = /* language=TypeScript */ r#"
            const mid = 1;
            const out = `x${mid}` + "y";
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const mid = 1;
        const out = `x${mid}y`;
        ");
	}

	#[test]
	fn folds_identifier_plus_template_literal() {
		let code = /* language=TypeScript */ r#"
            const head = "x";
            const mid = 1;
            const out = head + `${mid}y`;
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const head = 'x';
        const mid = 1;
        const out = `${head}${mid}y`;
        ");
	}

	#[test]
	fn folds_template_literal_plus_identifier() {
		let code = /* language=TypeScript */ r#"
            const mid = 1;
            const tail = "y";
            const out = `x${mid}` + tail;
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const mid = 1;
        const tail = 'y';
        const out = `x${mid}${tail}`;
        ");
	}

	#[test]
	fn folds_template_literal_plus_template_literal() {
		let code = /* language=TypeScript */ r#"
            const left = 1;
            const right = 2;
            const out = `x${left}` + `${right}y`;
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const left = 1;
        const right = 2;
        const out = `x${left}${right}y`;
        ");
	}
	#[test]
	fn folds_template_literals_with_many_vars() {
		let code = r#"
            const a = 1;
            const b = 2;
            let c = 3;
            const out = `#${a}#` + b + c + `#${a}#`;
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const a = 1;
        const b = 2;
        let c = 3;
        const out = `#${a}#${b}${c}#${a}#`;
        ");
	}

	#[test]
	fn folds_bigint_left_shift() {
		let code = /* language=TypeScript */ r#"
            const value = 1n << 20n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 1048576n;
        console.log(value);
        ");
	}

	#[test]
	fn folds_bigint_addition() {
		let code = /* language=TypeScript */ r#"
            const value = 123n + 456n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 579n;
        console.log(value);
        ");
	}

	#[test]
	fn folds_bigint_multiplication() {
		let code = /* language=TypeScript */ r#"
            const value = 100n * 200n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		// Currently not supported - multiplication operations on BigInts are not folded
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 100n * 200n;
        console.log(value);
        ");
	}

	#[test]
	fn folds_bigint_subtraction() {
		let code = /* language=TypeScript */ r#"
            const value = 1000n - 42n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		// Currently not supported - subtraction operations on BigInts are not folded
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 1000n - 42n;
        console.log(value);
        ");
	}

	#[test]
	fn folds_bigint_bitwise_or() {
		let code = /* language=TypeScript */ r#"
            const value = 5n | 3n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 7n;
        console.log(value);
        ");
	}

	#[test]
	fn folds_bigint_bitwise_and() {
		let code = /* language=TypeScript */ r#"
            const value = 5n & 3n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 1n;
        console.log(value);
        ");
	}

	#[test]
	fn folds_large_bigint_operations() {
		let code = /* language=TypeScript */ r#"
            const value = 9007199254740991n + 1n;
            console.log(value);
        "#;
		let out = test_pass!(code, FoldBinaryExpressionsPass);
		assert_snapshot!(out, /* language=TypeScript */ @"
        const value = 9007199254740992n;
        console.log(value);
        ");
	}
}
