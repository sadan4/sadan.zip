use miette::miette;
use oxc::ast::ast::{
	ArrayExpressionElement,
	Expression,
	ObjectPropertyKind,
	PropertyKey,
	UnaryOperator,
};
use serde_json::{Map as JM, Number as JN, Value as JV};

use crate::diag::{self, PResult};

#[expect(clippy::too_many_lines)]
pub fn ast2json(obj: &Expression) -> PResult<JV> {
	match obj {
		Expression::BooleanLiteral(v) => Ok(JV::Bool(v.value)),
		Expression::NullLiteral(_) => Ok(JV::Null),
		Expression::NumericLiteral(num) => match JN::from_f64(num.value) {
			Some(n) => Ok(JV::Number(n)),
			None => {
				Err(diag::err(obj, "numeric literal is out of range for JSON"))
			}
		},
		Expression::BigIntLiteral(b) => {
			let b = b.as_ref();
			debug_assert!(
				b.value
					.bytes()
					.all(|b| b.is_ascii_digit())
			);

			// TODO: use u128?
			b.value
				.parse::<u64>()
				.map_err(|e| {
					diag::err(b, "Failed to parse bigint literal as u64")
						.s(miette!(e))
				})
				.map(|v| JV::Number(JN::from(v)))
		}
		Expression::StringLiteral(s) => Ok(JV::String(String::from(s.value))),
		Expression::TemplateLiteral(t) if t.is_no_substitution_template() => {
			let str = t
				.as_ref()
				.single_quasi()
				.expect("it's a no sub template");
			Ok(JV::String(String::from(str)))
		}
		Expression::ArrayExpression(arr) => {
			let mut vec = Vec::with_capacity(arr.elements.len());
			for el in &arr.elements {
				match el {
					ArrayExpressionElement::SpreadElement(s) => {
						return Err(diag::err(
							s.as_ref(),
							"spread elements are not supported in JSON",
						));
					}
					ArrayExpressionElement::Elision(e) => {
						return Err(diag::err(
							e.as_ref(),
							"elided array elements are not supported in JSON",
						));
					}
					el => {
						let expr = el.as_expression().expect(
							"non-spread, non-elision element is an expression",
						);
						vec.push(ast2json(expr)?);
					}
				}
			}
			Ok(JV::Array(vec))
		}
		Expression::ObjectExpression(obj) => {
			let mut map = JM::with_capacity(obj.properties.len());
			for prop in &obj.properties {
				let (k, v) = match prop {
					ObjectPropertyKind::SpreadProperty(s) => {
						return Err(diag::err(
							s.as_ref(),
							"spread properties are not supported in JSON",
						));
					}
					ObjectPropertyKind::ObjectProperty(o) => {
						let key = match &o.key {
							PropertyKey::StaticIdentifier(i) => {
								i.name.to_string()
							}
							PropertyKey::StringLiteral(i) => {
								i.value.to_string()
							}
							other => {
								return Err(diag::err(
									other,
									"expected property key to be a StaticIdentifier or StringLiteral",
								));
							}
						};
						let value = ast2json(&o.value)?;
						(key, value)
					}
				};
				map.insert(k, v);
			}
			Ok(JV::Object(map))
		}
		Expression::ParenthesizedExpression(p) => {
			ast2json(&p.as_ref().expression)
		}
		Expression::UnaryExpression(e)
			if e.operator == UnaryOperator::UnaryNegation =>
		{
			if let JV::Number(n) = ast2json(&e.argument)? {
				Ok(JV::Number(
					JN::from_f64(-n.as_f64().expect("invalid number"))
						.expect("invalid number 2"),
				))
			} else {
				Err(diag::err(
					&e.argument,
					"Expected argument of unary negation to resolve to a number",
				))
			}
		}
		other => {
			Err(diag::err(other, "Expected expression to be a JSON value"))
		}
	}
}

#[cfg(test)]
mod tests {
	use ast_parser::parse_no_sema;
	use oxc::{ast::ast::Statement, span::SourceType};
	use oxc_allocator::Allocator;
	use serde_json::{Value as JV, json};

	use super::ast2json;
	use crate::diag::ParserDiagnostic;

	/// Parse `src` as a single expression statement and run it through
	/// [`parse_json`].
	fn run(src: &str) -> Result<JV, ParserDiagnostic> {
		let alloc = Allocator::new();
		let prog = parse_no_sema(&alloc, src, SourceType::mjs())
			.expect("source should parse without syntax errors");
		let stmt = prog
			.body
			.first()
			.expect("expected at least one statement");
		let Statement::ExpressionStatement(expr) = stmt else {
			panic!("expected an expression statement, got {stmt:?}");
		};
		ast2json(&expr.expression)
	}

	fn ok(src: &str) -> JV {
		run(src).expect("expected a JSON value")
	}

	fn err(src: &str) -> String {
		run(src)
			.expect_err("expected an error")
			.msg
			.into_owned()
	}

	#[test]
	fn boolean() {
		assert_eq!(ok("true"), json!(true));
		assert_eq!(ok("false"), json!(false));
	}

	#[test]
	fn null() {
		assert_eq!(ok("null"), JV::Null);
	}

	#[test]
	fn numeric_literal() {
		assert_eq!(ok("0"), json!(0.0));
		assert_eq!(ok("42"), json!(42.0));
		assert_eq!(ok("3.5"), json!(3.5));
		assert_eq!(ok("1e3"), json!(1000.0));
	}

	#[test]
	fn numeric_out_of_range() {
		// `1e999` parses to f64 infinity, which JSON cannot represent.
		assert_eq!(err("1e999"), "numeric literal is out of range for JSON");
	}

	#[test]
	fn bigint_literal() {
		assert_eq!(ok("0n"), json!(0));
		assert_eq!(ok("123n"), json!(123));
		assert_eq!(ok("18446744073709551615n"), json!(u64::MAX));
	}

	#[test]
	fn bigint_out_of_range() {
		// One past u64::MAX fails the u64 parse.
		assert_eq!(
			err("18446744073709551616n"),
			"Failed to parse bigint literal as u64"
		);
	}

	#[test]
	fn string_literal() {
		// wrapped in parens so a leading string is not parsed as a directive
		assert_eq!(ok(r#"("hello")"#), json!("hello"));
		assert_eq!(ok("('')"), json!(""));
	}

	#[test]
	fn template_literal_no_substitution() {
		assert_eq!(ok("`hello`"), json!("hello"));
	}

	#[test]
	fn template_literal_with_substitution_rejected() {
		assert_eq!(err("`hi ${x}`"), "Expected expression to be a JSON value");
	}

	#[test]
	fn object_expression() {
		assert_eq!(
			ok(r#"({ a: 1, "b": "two", c: true })"#),
			json!({ "a": 1.0, "b": "two", "c": true }),
		);
	}

	#[test]
	fn nested_object() {
		assert_eq!(
			ok(r#"({ a: { b: -1 }, c: "x" })"#),
			json!({ "a": { "b": -1.0 }, "c": "x" }),
		);
	}

	#[test]
	fn empty_object() {
		assert_eq!(ok("({})"), json!({}));
	}

	#[test]
	fn object_spread_rejected() {
		assert_eq!(
			err("({ ...a })"),
			"spread properties are not supported in JSON"
		);
	}

	#[test]
	fn object_computed_key_rejected() {
		assert_eq!(
			err("({ [a]: 1 })"),
			"expected property key to be a StaticIdentifier or StringLiteral"
		);
	}

	#[test]
	fn parenthesized() {
		assert_eq!(ok("(((5)))"), json!(5.0));
	}

	#[test]
	fn unary_negation() {
		assert_eq!(ok("-5"), json!(-5.0));
		assert_eq!(ok("-3.5"), json!(-3.5));
	}

	#[test]
	fn unary_negation_of_non_number_rejected() {
		assert_eq!(
			err(r#"-"foo""#),
			"Expected argument of unary negation to resolve to a number"
		);
	}

	#[test]
	fn array() {
		assert_eq!(ok("[]"), json!([]));
		assert_eq!(ok("[1, 2, 3]"), json!([1.0, 2.0, 3.0]));
		assert_eq!(
			ok(r#"[1, "two", true, null, [-3]]"#),
			json!([1.0, "two", true, JV::Null, [-3.0]]),
		);
		assert_eq!(
			ok("[{ a: 1 }, { b: [2] }]"),
			json!([{ "a": 1.0 }, { "b": [2.0] }]),
		);
	}

	#[test]
	fn array_spread_rejected() {
		assert_eq!(err("[...a]"), "spread elements are not supported in JSON");
	}

	#[test]
	fn array_elision_rejected() {
		assert_eq!(
			err("[1, , 2]"),
			"elided array elements are not supported in JSON"
		);
	}

	#[test]
	fn identifier_rejected() {
		assert_eq!(err("foo"), "Expected expression to be a JSON value");
	}
}
