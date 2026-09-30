use oxc::ast::ast::{Expression, ObjectExpression, UnaryOperator};
use parser_diag::{PResult, err};
use serde_json::{Number, Value};

impl<'ast> super::WebpackAstParser<'ast> {
	pub(crate) fn obj_to_json(
		obj: &'ast ObjectExpression<'ast>,
	) -> PResult<Value> {
		let mut map = serde_json::Map::new();
		for prop in &obj.properties {
			let prop = prop
				.as_property()
				.ok_or_else(|| err(prop, "Can't convert spread to json"))?;
			let key = prop.key.static_name().ok_or_else(|| {
				err(prop, "Can't convert non-identifier key to json")
			})?;
			map.insert(key.to_string(), Self::expr_to_json(&prop.value)?);
		}
		Ok(Value::Object(map))
	}
	pub(crate) fn expr_to_json(expr: &'ast Expression<'ast>) -> PResult<Value> {
		let ret = match expr {
			Expression::BooleanLiteral(lit) => Value::Bool(lit.value),
			Expression::NullLiteral(_) => Value::Null,
			Expression::NumericLiteral(lit) => Value::Number(
				Number::from_f64(lit.value)
					.ok_or_else(|| err(&**lit, "Invalid number"))?,
			),
			Expression::StringLiteral(lit) => {
				Value::String(lit.value.to_string())
			}
			Expression::TemplateLiteral(lit) => {
				if lit.is_no_substitution_template() {
					Value::String(
						lit.quasis[0]
							.value
							.cooked
							.unwrap()
							.to_string(),
					)
				} else {
					return Err(err(
						&**lit,
						"Can't convert template literal with substitutions to json",
					));
				}
			}
			Expression::ArrayExpression(arr) => {
				let mut vec = Vec::with_capacity(arr.elements.len());
				for elem in &arr.elements {
					let elem = elem.as_expression().ok_or_else(|| {
						err(elem, "Array element is not an expression")
					})?;
					vec.push(Self::expr_to_json(elem)?);
				}
				Value::Array(vec)
			}
			Expression::ObjectExpression(object_expression) => {
				Self::obj_to_json(object_expression)?
			}
			Expression::ParenthesizedExpression(parenthesized_expression) => {
				Self::expr_to_json(&parenthesized_expression.expression)?
			}
			Expression::UnaryExpression(unary) => {
				let arg = Self::expr_to_json(&unary.argument)?;
				match unary.operator {
					UnaryOperator::LogicalNot => Value::Bool(!is_truthy(&arg)),
					UnaryOperator::UnaryNegation => {
						let n = arg.as_f64().ok_or_else(|| {
							err(&unary.argument, "Can't negate non-number")
						})?;
						Value::Number(
							Number::from_f64(-n).ok_or_else(|| {
								err(&**unary, "Invalid number")
							})?,
						)
					}
					UnaryOperator::UnaryPlus if arg.is_number() => arg,
					UnaryOperator::UnaryPlus => {
						return Err(err(
							&unary.argument,
							"Can't convert unary plus on non-number to json",
						));
					}
					UnaryOperator::Void => Value::Null,
					op => {
						return Err(err(
							&**unary,
							format!(
								"Can't convert unary `{}` to json",
								op.as_str()
							),
						));
					}
				}
			}
			other => {
				return Err(err(other, "Can't convert expression to json"));
			}
		};
		Ok(ret)
	}
}

/// JS truthiness of a json value
fn is_truthy(v: &Value) -> bool {
	match v {
		Value::Null => false,
		Value::Bool(b) => *b,
		Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
		Value::String(s) => !s.is_empty(),
		Value::Array(_) | Value::Object(_) => true,
	}
}

#[cfg(test)]
mod tests {
	use oxc::{allocator::Allocator, parser::Parser, span::SourceType};
	use serde_json::{Value, json};

	use crate::WebpackAstParser;

	fn to_json(src: &str) -> Option<Value> {
		let alloc = Allocator::new();
		let expr = Parser::new(&alloc, src, SourceType::cjs())
			.parse_expression()
			.unwrap();
		let expr = alloc.alloc(expr);
		WebpackAstParser::expr_to_json(expr).ok()
	}


	#[test]
	fn void_0() {
		assert_eq!(to_json("void 0"), Some(json!(null)));
	}

	#[test]
	fn unary_not() {
		assert_eq!(to_json("!0"), Some(json!(true)));
		assert_eq!(to_json("!1"), Some(json!(false)));
		assert_eq!(to_json(r#"!"""#), Some(json!(true)));
		assert_eq!(to_json("!{}"), Some(json!(false)));
		assert_eq!(to_json("!!null"), Some(json!(false)));
	}

	#[test]
	fn unary_numbers() {
		assert_eq!(to_json("-1"), Some(json!(-1.0)));
		assert_eq!(to_json("+1.5"), Some(json!(1.5)));
		assert_eq!(to_json("-(-2)"), Some(json!(2.0)));
	}

	#[test]
	fn unary_unsupported() {
		assert_eq!(to_json("void 0"), None);
		assert_eq!(to_json("typeof 1"), None);
		assert_eq!(to_json(r#"-"a""#), None);
		assert_eq!(to_json("+true"), None);
	}
}
