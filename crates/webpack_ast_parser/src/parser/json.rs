use oxc::ast::ast::Expression;
use parser_diag::PResult;
use serde_json::{Map, Number, Value};

use super::value_tracking::{ConstantPropertyKey, ConstantValue, SpannedValue};

impl<'ast> super::WebpackAstParser<'ast> {
	/// Resolves the constant value of `expr` and converts it to json
	#[expect(clippy::future_not_send)]
	pub(crate) async fn expr_to_json(
		&self,
		expr: &'ast Expression<'ast>,
	) -> PResult<Value> {
		let value = self
			.resolve_constant_value(expr)
			.await?;
		match value_to_json(&value) {
			Ok(json) => Ok(json),
			Err((at, msg)) => Err(self
				.remote_err(&at.span, at.module_id, msg)
				.await),
		}
	}
}

/// Converts a constant value to json, returning the value that can't be
/// converted and why on failure
fn value_to_json(
	v: &SpannedValue,
) -> Result<Value, (&SpannedValue, &'static str)> {
	Ok(match &v.value {
		ConstantValue::Number(n) => Value::Number(
			Number::from_f64(**n)
				.ok_or((v, "`Infinity` can't be converted to json"))?,
		),
		ConstantValue::NaN => {
			return Err((v, "`NaN` can't be converted to json"));
		}
		ConstantValue::BigInt(_) => {
			return Err((v, "bigints can't be converted to json"));
		}
		ConstantValue::Boolean(b) => Value::Bool(*b),
		ConstantValue::String(s) => Value::String(s.clone()),
		ConstantValue::Array(a) => array(a)?,
		// sets have no json form, an array of their elements is the closest
		ConstantValue::Set(s) => array(s)?,
		ConstantValue::Object(props) => {
			// JS orders integer keys first
			let (numbers, strings): (Vec<_>, Vec<_>) =
				props.iter().partition(|(k, _)| {
					matches!(k, ConstantPropertyKey::Number(_))
				});
			let mut map = Map::with_capacity(props.len());
			for (k, v) in numbers.into_iter().chain(strings) {
				map.insert(k.to_string(), value_to_json(v)?);
			}
			Value::Object(map)
		}
		// `undefined` has no json form, `null` is the closest
		ConstantValue::Undefined | ConstantValue::Null => Value::Null,
	})
}

/// Converts `elts` to a json array
fn array<'v>(
	elts: impl IntoIterator<Item = &'v SpannedValue>,
) -> Result<Value, (&'v SpannedValue, &'static str)> {
	elts.into_iter()
		.map(value_to_json)
		.collect::<Result<_, _>>()
		.map(Value::Array)
}

#[cfg(test)]
mod tests {
	#![allow(clippy::future_not_send)]

	use macros::test;
	use oxc::allocator::Allocator;
	use serde_json::{Value, json};

	use crate::WebpackAstParser;

	async fn to_json(src: &str) -> Option<Value> {
		to_json_with("", src).await
	}

	/// converts `src` to json, with `prelude` in scope
	async fn to_json_with(prelude: &str, src: &str) -> Option<Value> {
		let alloc = Allocator::new();
		let source = format!(
			"// Webpack Module 123\n0,function(e, t, n) {{ {prelude}\nconst \
			 x = {src}; }}"
		);
		let parser = WebpackAstParser::try_new(&alloc, &source).unwrap();
		let scoping = parser.sema.scoping();
		let x = scoping
			.symbol_ids()
			.find(|&id| scoping.symbol_name(id) == "x")
			.unwrap();
		let init = parser.constant_value_of(&x).unwrap();
		parser.expr_to_json(init).await.ok()
	}

	#[test]
	async fn void_0() {
		assert_eq!(to_json("void 0").await, Some(json!(null)));
		assert_eq!(to_json("undefined").await, Some(json!(null)));
	}

	#[test]
	async fn unary_not() {
		assert_eq!(to_json("!0").await, Some(json!(true)));
		assert_eq!(to_json("!1").await, Some(json!(false)));
		assert_eq!(to_json(r#"!"""#).await, Some(json!(true)));
		assert_eq!(to_json("!{}").await, Some(json!(false)));
		assert_eq!(to_json("!!null").await, Some(json!(false)));
	}

	#[test]
	async fn unary_numbers() {
		assert_eq!(to_json("-1").await, Some(json!(-1.0)));
		assert_eq!(to_json("+1.5").await, Some(json!(1.5)));
		assert_eq!(to_json("-(-2)").await, Some(json!(2.0)));
		assert_eq!(to_json("+true").await, Some(json!(1.0)));
		assert_eq!(to_json("typeof 1").await, Some(json!("number")));
	}

	#[test]
	async fn not_json() {
		assert_eq!(to_json(r#"-"a""#).await, None);
		assert_eq!(to_json("NaN").await, None);
		assert_eq!(to_json("1 / 0").await, None);
		assert_eq!(to_json("1n").await, None);
		assert_eq!(to_json("[1n]").await, None);
	}

	#[test]
	async fn sets_are_arrays() {
		assert_eq!(to_json("new Set([1, 1])").await, Some(json!([1.0])));
	}

	#[test]
	async fn numeric_keys_first() {
		let json = to_json(r"{ b: 1, 1: 2, a: 3 }")
			.await
			.unwrap();
		let keys = json
			.as_object()
			.unwrap()
			.keys()
			.collect::<Vec<_>>();
		assert_eq!(keys, ["1", "a", "b"]);
	}

	#[test]
	async fn resolves_declared_constant() {
		assert_eq!(to_json_with("let a = 1;", "a").await, Some(json!(1.0)));
		assert_eq!(
			to_json_with("const a = \"s\";", "a").await,
			Some(json!("s"))
		);
		assert_eq!(to_json_with("var a = !0;", "a").await, Some(json!(true)));
	}

	#[test]
	async fn resolves_constant_assigned_once() {
		assert_eq!(
			to_json_with("let a;\na = [1];", "a").await,
			Some(json!([1.0]))
		);
	}

	#[test]
	async fn resolves_constant_chain() {
		assert_eq!(
			to_json_with("let a = 1, b = a;", "b").await,
			Some(json!(1.0))
		);
	}

	#[test]
	async fn resolves_nested_constants() {
		assert_eq!(
			to_json_with(
				"let a = 1, b = { c: !1 };",
				"{ a: a, b, arr: [a, -a] }"
			)
			.await,
			Some(json!({ "a": 1.0, "b": { "c": false }, "arr": [1.0, -1.0] }))
		);
	}

	#[test]
	async fn does_not_resolve_non_constants() {
		// undeclared global
		assert_eq!(to_json("a").await, None);
		// written more than once
		assert_eq!(to_json_with("let a;\na = 1;\na = 2;", "a").await, None);
		// compound assignment
		assert_eq!(to_json_with("let a;\na += 1;", "a").await, None);
		// not a variable declaration
		assert_eq!(to_json_with("function a() {}", "a").await, None);
		// initializer can't be converted
		assert_eq!(to_json_with("let a = foo();", "a").await, None);
		// reassigned after initializer
		assert_eq!(to_json_with("let a = 1;\na = 2;", "a").await, None);
	}

	#[test]
	async fn does_not_resolve_cyclic_constants() {
		assert_eq!(to_json_with("var a = a;", "a").await, None);
		assert_eq!(to_json_with("var a = b, b = a;", "a").await, None);
		assert_eq!(to_json_with("var a = [b], b = { a };", "a").await, None);
	}

	#[test]
	async fn resolves_repeated_non_cyclic_constants() {
		assert_eq!(
			to_json_with("let a = 1, b = [a, a];", "[b, b]").await,
			Some(json!([[1.0, 1.0], [1.0, 1.0]]))
		);
	}

	#[test]
	async fn object_spread() {
		assert_eq!(
			to_json("{ ...{ a: 1 }, b: 2 }").await,
			Some(json!({ "a": 1.0, "b": 2.0 }))
		);
		assert_eq!(
			to_json_with("let o = { a: 1 };", "{ ...o, b: 2 }").await,
			Some(json!({ "a": 1.0, "b": 2.0 }))
		);
		assert_eq!(to_json("{ ...{} }").await, Some(json!({})));
		assert_eq!(to_json("{ ...null }").await, Some(json!({})));
		assert_eq!(to_json("{ ...undefined }").await, Some(json!({})));
		assert_eq!(to_json("{ ...void 0 }").await, Some(json!({})));
		assert_eq!(to_json("{ ...[1] }").await, Some(json!({ "0": 1.0 })));
		assert_eq!(
			to_json(r#"{ ..."ab" }"#).await,
			Some(json!({ "0": "a", "1": "b" }))
		);
	}

	#[test]
	async fn object_spread_nested() {
		assert_eq!(
			to_json_with(
				"let a = { x: 1 }, b = { ...a, y: 2 };",
				"{ ...b, z: 3 }"
			)
			.await,
			Some(json!({ "x": 1.0, "y": 2.0, "z": 3.0 }))
		);
	}

	#[test]
	async fn object_spread_later_keys_win() {
		let prelude = "let o = { a: 1, b: 1 };";
		assert_eq!(
			to_json_with(prelude, "{ a: 2, ...o }").await,
			Some(json!({ "a": 1.0, "b": 1.0 }))
		);
		assert_eq!(
			to_json_with(prelude, "{ ...o, a: 2 }").await,
			Some(json!({ "a": 2.0, "b": 1.0 }))
		);
		assert_eq!(
			to_json_with(prelude, "{ ...o, ...{ b: 3 } }").await,
			Some(json!({ "a": 1.0, "b": 3.0 }))
		);
	}

	#[test]
	async fn object_spread_unsupported() {
		// can't be resolved
		assert_eq!(to_json("{ ...o }").await, None);
		assert_eq!(to_json("{ ...foo() }").await, None);
		// cyclic
		assert_eq!(to_json_with("var o = { ...o };", "o").await, None);
	}
}
