use ast_parser::AstParser as _;
use oxc::{
	ast::ast::{
		Expression,
		IdentifierReference,
		ObjectExpression,
		ObjectPropertyKind,
		UnaryOperator,
	},
	semantic::SymbolId,
};
use parser_diag::{PResult, err};
use serde_json::{Number, Value};

impl<'ast> super::WebpackAstParser<'ast> {
	pub(crate) fn expr_to_json(
		&self,
		expr: &'ast Expression<'ast>,
	) -> PResult<Value> {
		self.expr_to_json_impl(expr, &mut Vec::new())
	}
	fn obj_to_json(
		&self,
		obj: &'ast ObjectExpression<'ast>,
		resolving: &mut Vec<SymbolId>,
	) -> PResult<Value> {
		let mut map = serde_json::Map::new();
		for prop in &obj.properties {
			match &prop {
				ObjectPropertyKind::SpreadProperty(spread) => {
					let spread_value = self
						.expr_to_json_impl(&spread.argument, resolving)
						.map_err(|e| {
							err(
								&**spread,
								"Can't conver spread expression to JSON",
							)
							.s(e)
						})?;
					if let Value::Object(spread_map) = spread_value {
						map.extend(spread_map);
					} else {
						return Err(err(
							&**spread,
							"Spread expression must evaluate to an object",
						));
					}
				}
				ObjectPropertyKind::ObjectProperty(prop) => {
					let key = prop.key.static_name().ok_or_else(|| {
						err(&**prop, "Can't convert non-identifier key to json")
					})?;
					map.insert(
						key.to_string(),
						self.expr_to_json_impl(&prop.value, resolving)?,
					);
				}
			}
		}
		Ok(Value::Object(map))
	}
	/// `resolving` is the stack of identifiers whose constant values are
	/// currently being converted, used to detect cycles
	fn expr_to_json_impl(
		&self,
		expr: &'ast Expression<'ast>,
		resolving: &mut Vec<SymbolId>,
	) -> PResult<Value> {
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
					vec.push(self.expr_to_json_impl(elem, resolving)?);
				}
				Value::Array(vec)
			}
			Expression::ObjectExpression(object_expression) => {
				self.obj_to_json(object_expression, resolving)?
			}
			Expression::ParenthesizedExpression(parenthesized_expression) => {
				self.expr_to_json_impl(
					&parenthesized_expression.expression,
					resolving,
				)?
			}
			Expression::UnaryExpression(unary) => {
				let arg = self.expr_to_json_impl(&unary.argument, resolving)?;
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
			Expression::Identifier(ident) => {
				self.ident_to_json(ident, resolving)?
			}
			other => {
				return Err(err(other, "Can't convert expression to json"));
			}
		};
		Ok(ret)
	}
	fn ident_to_json(
		&self,
		ident: &'ast IdentifierReference<'ast>,
		resolving: &mut Vec<SymbolId>,
	) -> PResult<Value> {
		let Some((sym_id, value)) = self
			.sym_id_of(ident)
			.zip(self.constant_value_of(ident))
		else {
			return Err(err(
				ident,
				"Couldn't resolve constant value of identifier to convert to json",
			));
		};
		if resolving.contains(&sym_id) {
			return Err(err(
				ident,
				"Cycle detected while resolving constant value of identifier",
			));
		}
		resolving.push(sym_id);
		let ret = self.expr_to_json_impl(value, resolving);
		resolving.pop();
		ret
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
	use oxc::{
		allocator::Allocator,
		ast::ast::{Expression, Statement},
	};
	use serde_json::{Value, json};

	use crate::WebpackAstParser;

	fn to_json(src: &str) -> Option<Value> {
		to_json_with("", src)
	}

	/// converts `src` to json, with `prelude` in scope
	fn to_json_with(prelude: &str, src: &str) -> Option<Value> {
		let alloc = Allocator::new();
		let src = alloc.alloc_str(&format!("{prelude}\nx = {src};"));
		let parser = WebpackAstParser::try_new(&alloc, src).unwrap();
		let Some(Statement::ExpressionStatement(stmt)) =
			parser.prog.body.last()
		else {
			panic!("expected an expression statement");
		};
		let Expression::AssignmentExpression(assign) = &stmt.expression else {
			panic!("expected an assignment expression");
		};
		parser.expr_to_json(&assign.right).ok()
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
		assert_eq!(to_json("typeof 1"), None);
		assert_eq!(to_json(r#"-"a""#), None);
		assert_eq!(to_json("+true"), None);
	}

	#[test]
	fn resolves_declared_constant() {
		assert_eq!(to_json_with("let a = 1;", "a"), Some(json!(1.0)));
		assert_eq!(to_json_with("const a = \"s\";", "a"), Some(json!("s")));
		assert_eq!(to_json_with("var a = !0;", "a"), Some(json!(true)));
	}

	#[test]
	fn resolves_constant_assigned_once() {
		assert_eq!(to_json_with("let a;\na = [1];", "a"), Some(json!([1.0])));
	}

	#[test]
	fn resolves_constant_chain() {
		assert_eq!(to_json_with("let a = 1, b = a;", "b"), Some(json!(1.0)));
	}

	#[test]
	fn resolves_nested_constants() {
		assert_eq!(
			to_json_with(
				"let a = 1, b = { c: !1 };",
				"{ a: a, b, arr: [a, -a] }"
			),
			Some(json!({ "a": 1.0, "b": { "c": false }, "arr": [1.0, -1.0] }))
		);
	}

	#[test]
	fn does_not_resolve_non_constants() {
		// undeclared global
		assert_eq!(to_json("a"), None);
		// written more than once
		assert_eq!(to_json_with("let a;\na = 1;\na = 2;", "a"), None);
		// compound assignment
		assert_eq!(to_json_with("let a;\na += 1;", "a"), None);
		// not a variable declaration
		assert_eq!(to_json_with("function a() {}", "a"), None);
		// initializer can't be converted
		assert_eq!(to_json_with("let a = foo();", "a"), None);
		// reassigned after initializer
		assert_eq!(to_json_with("let a = 1;\na = 2;", "a"), None);
	}

	#[test]
	fn does_not_resolve_cyclic_constants() {
		assert_eq!(to_json_with("var a = a;", "a"), None);
		assert_eq!(to_json_with("var a = b, b = a;", "a"), None);
		assert_eq!(to_json_with("var a = [b], b = { a };", "a"), None);
	}

	#[test]
	fn resolves_repeated_non_cyclic_constants() {
		assert_eq!(
			to_json_with("let a = 1, b = [a, a];", "[b, b]"),
			Some(json!([[1.0, 1.0], [1.0, 1.0]]))
		);
	}

	#[test]
	fn object_spread() {
		assert_eq!(
			to_json("{ ...{ a: 1 }, b: 2 }"),
			Some(json!({ "a": 1.0, "b": 2.0 }))
		);
		assert_eq!(
			to_json_with("let o = { a: 1 };", "{ ...o, b: 2 }"),
			Some(json!({ "a": 1.0, "b": 2.0 }))
		);
		assert_eq!(to_json("{ ...{} }"), Some(json!({})));
	}

	#[test]
	fn object_spread_nested() {
		assert_eq!(
			to_json_with(
				"let a = { x: 1 }, b = { ...a, y: 2 };",
				"{ ...b, z: 3 }"
			),
			Some(json!({ "x": 1.0, "y": 2.0, "z": 3.0 }))
		);
	}

	#[test]
	fn object_spread_later_keys_win() {
		let prelude = "let o = { a: 1, b: 1 };";
		assert_eq!(
			to_json_with(prelude, "{ a: 2, ...o }"),
			Some(json!({ "a": 1.0, "b": 1.0 }))
		);
		assert_eq!(
			to_json_with(prelude, "{ ...o, a: 2 }"),
			Some(json!({ "a": 2.0, "b": 1.0 }))
		);
		assert_eq!(
			to_json_with(prelude, "{ ...o, ...{ b: 3 } }"),
			Some(json!({ "a": 1.0, "b": 3.0 }))
		);
	}

	#[test]
	fn object_spread_unsupported() {
		// not an object
		assert_eq!(to_json("{ ...[1] }"), None);
		assert_eq!(to_json("{ ...null }"), None);
		assert_eq!(to_json(r#"{ ..."ab" }"#), None);
		// can't be resolved
		assert_eq!(to_json("{ ...o }"), None);
		assert_eq!(to_json("{ ...foo() }"), None);
		// cyclic
		assert_eq!(to_json_with("var o = { ...o };", "o"), None);
	}
}
