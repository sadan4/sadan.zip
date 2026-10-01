#![allow(clippy::future_not_send)]

use super::*;
use crate::{bundle::IModuleCache, sync::ThreadSafeParser};
use anyhow::Context as _;
use async_trait::async_trait;
use macros::test;
use oxc::allocator::Allocator;
use std::collections::HashMap;
use url::Url;

const MODULE_ID: u32 = 123_456;

/// A [`ConstantValue`] with all span information stripped, for easy
/// comparisons
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Plain(ConstantValue<Self>);

impl From<SpannedValue> for Plain {
	fn from(v: SpannedValue) -> Self {
		Self(match v.value {
			ConstantValue::Number(n) => ConstantValue::Number(n),
			ConstantValue::NaN => ConstantValue::NaN,
			ConstantValue::BigInt(n) => ConstantValue::BigInt(n),
			ConstantValue::Boolean(b) => ConstantValue::Boolean(b),
			ConstantValue::String(s) => ConstantValue::String(s),
			ConstantValue::Array(a) => {
				ConstantValue::Array(a.into_iter().map(Self::from).collect())
			}
			ConstantValue::Object(o) => ConstantValue::Object(
				o.into_iter()
					.map(|(k, v)| (k, Self::from(v)))
					.collect(),
			),
			ConstantValue::Set(s) => {
				ConstantValue::Set(s.into_iter().map(Self::from).collect())
			}
			ConstantValue::Undefined => ConstantValue::Undefined,
			ConstantValue::Null => ConstantValue::Null,
		})
	}
}

fn num(n: f64) -> Plain {
	Plain(ConstantValue::from(n))
}
fn string(s: &str) -> Plain {
	Plain(ConstantValue::String(s.to_owned()))
}
fn boolean(b: bool) -> Plain {
	Plain(ConstantValue::Boolean(b))
}
fn array(elts: impl IntoIterator<Item = Plain>) -> Plain {
	Plain(ConstantValue::Array(elts.into_iter().collect()))
}
fn object<'a>(props: impl IntoIterator<Item = (&'a str, Plain)>) -> Plain {
	Plain(ConstantValue::Object(
		props
			.into_iter()
			.map(|(k, v)| (ConstantPropertyKey::String(k.to_owned()), v))
			.collect(),
	))
}

/// Wraps `body` in a webpack module and resolves the constant value of the
/// initializer of the only symbol named `x`
async fn resolve_in(body: &str) -> PResult<SpannedValue> {
	resolve_in_modules(body, &[]).await
}

/// Wraps `body` in a webpack module with the id `id`
fn module_source(id: u32, body: &str) -> String {
	format!("// Webpack Module {id}\n0,function(e, t, n) {{ {body} }}")
}

/// A module cache that parses modules from their source on every request
#[derive(Clone)]
struct SourceCache(HashMap<ModuleId, Arc<str>>);

#[async_trait]
impl IModuleCache for SourceCache {
	async fn get_module_filepath(&self, _id: ModuleId) -> Option<Url> {
		None
	}
	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> anyhow::Result<Arc<ThreadSafeParser>> {
		let source = self
			.0
			.get(&id)
			.with_context(|| format!("no module {id} in the test cache"))?;
		let mut p = ThreadSafeParser::new(Arc::clone(source))
			.map_err(|e| anyhow::anyhow!("{e:?}"))?;
		p.set_module_cache(Arc::new(self.clone()));
		Ok(Arc::new(p))
	}
}

/// [`resolve_in`], with `modules` (`(id, body)` pairs) available to require
async fn resolve_in_modules(
	body: &str,
	modules: &[(u32, &str)],
) -> PResult<SpannedValue> {
	let alloc = Allocator::new();
	let source = module_source(MODULE_ID, body);
	let mut p = WebpackAstParser::try_new(&alloc, &source).unwrap();
	p.set_module_cache(Arc::new(SourceCache(
		modules
			.iter()
			.map(|&(id, body)| (ModuleId(id), module_source(id, body).into()))
			.collect(),
	)));
	let scoping = p.sema.scoping();
	let syms = scoping
		.symbol_ids()
		.filter(|&id| scoping.symbol_name(id) == "x")
		.collect::<Vec<_>>();
	let [sym] = syms.as_slice() else {
		panic!("expected exactly one symbol named `x`, found {syms:?}");
	};
	let init = p
		.constant_value_of(sym)
		.expect("`x` should have a constant initializer");
	p.resolve_constant_value(init).await
}

/// Resolves the constant value of `expr`
async fn resolve(expr: &str) -> PResult<SpannedValue> {
	resolve_in(&format!("const x = {expr};")).await
}

async fn resolve_plain(expr: &str) -> Plain {
	match resolve(expr).await {
		Ok(v) => v.into(),
		Err(e) => panic!("failed to resolve `{expr}`: {e:?}"),
	}
}

async fn assert_resolves(expr: &str, expected: Plain) {
	assert_eq!(resolve_plain(expr).await, expected, "for `{expr}`");
}

async fn assert_errors(expr: &str) {
	if let Ok(v) = resolve(expr).await {
		panic!(
			"expected `{expr}` to fail to resolve, got {:?}",
			Plain::from(v)
		);
	}
}

#[test]
async fn logical_expr() {
	assert_resolves("null ?? 1", num(1.0)).await;
	assert_resolves("false ?? 1", boolean(false)).await;
	assert_resolves("0 || 2", num(2.0)).await;
	assert_resolves("2 || 0", num(2.0)).await;
	assert_resolves("1 && 2", num(2.0)).await;
	assert_resolves("0 && 2", num(0.0)).await;
}

mod constant_value_from_f64 {
	use super::*;
	use macros::test;

	#[test]
	fn number() {
		assert_eq!(
			ConstantValue::<()>::from(1.5),
			ConstantValue::Number(NotNan::new(1.5).unwrap())
		);
	}

	#[test]
	fn nan() {
		assert_eq!(ConstantValue::<()>::from(f64::NAN), ConstantValue::NaN);
	}

	#[test]
	fn infinity() {
		assert_eq!(
			ConstantValue::<()>::from(f64::INFINITY),
			ConstantValue::Number(NotNan::new(f64::INFINITY).unwrap())
		);
	}
}

mod literals {
	use super::*;
	use macros::test;

	#[test]
	async fn booleans() {
		assert_resolves("true", boolean(true)).await;
		assert_resolves("false", boolean(false)).await;
	}

	#[test]
	async fn null() {
		assert_resolves("null", Plain(ConstantValue::Null)).await;
	}

	#[test]
	async fn numbers() {
		assert_resolves("1", num(1.0)).await;
		assert_resolves("1.5", num(1.5)).await;
		assert_resolves("0x10", num(16.0)).await;
		assert_resolves("1e3", num(1000.0)).await;
	}

	#[test]
	async fn bigint() {
		assert_resolves("10n", Plain(ConstantValue::BigInt(10.into()))).await;
	}

	#[test]
	async fn hex_bigint() {
		assert_resolves("0x10n", Plain(ConstantValue::BigInt(16.into()))).await;
	}

	#[test]
	async fn strings() {
		assert_resolves(r#""foo""#, string("foo")).await;
		assert_resolves("'bar'", string("bar")).await;
		assert_resolves(r#""""#, string("")).await;
		assert_resolves(r#""a\nb""#, string("a\nb")).await;
	}

	#[test]
	async fn no_substitution_template() {
		assert_resolves("`foo`", string("foo")).await;
		assert_resolves("`a\\nb`", string("a\nb")).await;
	}

	#[test]
	async fn parenthesized() {
		assert_resolves("(1)", num(1.0)).await;
		assert_resolves("((\"a\"))", string("a")).await;
	}
}

mod arrays {
	use super::*;
	use macros::test;

	#[test]
	async fn empty() {
		assert_resolves("[]", array([])).await;
	}

	#[test]
	async fn simple() {
		assert_resolves(
			r#"[1, "a", true, null]"#,
			array([
				num(1.0),
				string("a"),
				boolean(true),
				Plain(ConstantValue::Null),
			]),
		)
		.await;
	}

	#[test]
	async fn nested() {
		assert_resolves(
			"[[1], [[2]]]",
			array([array([num(1.0)]), array([array([num(2.0)])])]),
		)
		.await;
	}

	#[test]
	async fn spread() {
		assert_resolves(
			"[1, ...[2, 3], 4]",
			array([num(1.0), num(2.0), num(3.0), num(4.0)]),
		)
		.await;
	}

	#[test]
	async fn spread_empty() {
		assert_resolves("[...[]]", array([])).await;
	}

	#[test]
	async fn spread_non_array_errors() {
		assert_errors("[...1]").await;
		assert_errors(r#"[..."abc"]"#).await;
	}

	#[test]
	async fn elision_errors() {
		assert_errors("[1, , 2]").await;
	}

	#[test]
	async fn invalid_element_errors() {
		assert_errors("[1, /a/]").await;
	}
}

mod spans {
	use super::*;
	use macros::test;

	#[test]
	async fn literal_span_and_module_id() {
		let body = "const x = 12;";
		let v = resolve_in(body).await.unwrap();
		assert_eq!(v.module_id, ModuleId(MODULE_ID));
		let header =
			format!("// Webpack Module {MODULE_ID}\n0,function(e, t, n) {{ ");
		let start =
			u32::try_from(header.len() + body.find("12").unwrap()).unwrap();
		assert_eq!(v.span, Span::new(start, start + 2));
	}

	#[test]
	async fn array_element_spans() {
		let v = resolve("[1, 22]").await.unwrap();
		let ConstantValue::Array(elts) = &v.value else {
			panic!("expected array, got {v:?}");
		};
		let [a, b] = elts.as_slice() else {
			panic!("expected two elements, got {elts:?}");
		};
		assert_eq!(a.span.size(), 1);
		assert_eq!(b.span.size(), 2);
		assert!(v.span.contains_inclusive(a.span));
		assert!(v.span.contains_inclusive(b.span));
	}

	#[test]
	async fn parenthesized_uses_inner_span() {
		let v = resolve("(1)").await.unwrap();
		assert_eq!(v.span.size(), 1);
	}

	#[test]
	async fn missing_module_id_errors() {
		let alloc = Allocator::new();
		let source = "0,function(e, t, n) { const x = 1; }";
		let p = WebpackAstParser::try_new(&alloc, source).unwrap();
		let scoping = p.sema.scoping();
		let sym = scoping
			.symbol_ids()
			.find(|&id| scoping.symbol_name(id) == "x")
			.unwrap();
		let init = p.constant_value_of(&sym).unwrap();
		assert!(
			p.resolve_constant_value(init)
				.await
				.is_err()
		);
	}
}

mod unsupported {
	use super::*;
	use macros::test;

	#[test]
	async fn regex() {
		assert_errors("/abc/g").await;
	}

	#[test]
	async fn tagged_template() {
		assert_errors("String.raw`foo`").await;
	}

	#[test]
	async fn template_with_substitutions() {
		assert_errors("`a${1}b`").await;
	}

	#[test]
	async fn functions() {
		assert_errors("() => 1").await;
		assert_errors("function () { return 1; }").await;
		assert_errors("async () => 1").await;
	}

	#[test]
	async fn other_expressions() {
		assert_errors("this").await;
		assert_errors("class {}").await;
	}
}

#[test]
async fn sequence() {
	assert_resolves("(0, 1)", num(1.0)).await;
}

#[test]
async fn assignment() {
	assert_resolves_in("let y; const x = y = 5;", num(5.0)).await;
}

async fn assert_resolves_in(body: &str, expected: Plain) {
	match resolve_in(body).await {
		Ok(v) => assert_eq!(Plain::from(v), expected, "for `{body}`"),
		Err(e) => panic!("failed to resolve in `{body}`: {e:?}"),
	}
}

#[test]
async fn binary_arithmetic() {
	assert_resolves("1 + 2", num(3.0)).await;
	assert_resolves("2 * 3 - 1", num(5.0)).await;
	assert_resolves("1 << 4", num(16.0)).await;
	assert_resolves("0 / 0", Plain(ConstantValue::NaN)).await;
	assert_resolves("1 / 0", num(f64::INFINITY)).await;
}

#[test]
async fn identifier() {
	assert_resolves_in("const y = 5; const x = y;", num(5.0)).await;
}

#[test]
async fn identifier_chain() {
	assert_resolves_in(
		"const z = \"a\"; const y = z; const x = [y, y];",
		array([string("a"), string("a")]),
	)
	.await;
}

#[test]
async fn identifier_assigned_once() {
	assert_resolves_in("let y; y = 5; const x = y;", num(5.0)).await;
}

#[test]
async fn identifier_reassigned_errors() {
	assert!(
		resolve_in("let y = 1; y = 2; const x = y;")
			.await
			.is_err()
	);
}

#[test]
async fn undefined() {
	assert_resolves("void 0", Plain(ConstantValue::Undefined)).await;
	assert_resolves("undefined", Plain(ConstantValue::Undefined)).await;
}

#[test]
async fn nan_and_infinity() {
	assert_resolves("NaN", Plain(ConstantValue::NaN)).await;
	assert_resolves("Infinity", num(f64::INFINITY)).await;
}

#[test]
async fn await_expression() {
	assert_resolves_in("async function f() { const x = await 5; }", num(5.0))
		.await;
}

#[test]
async fn binary_string_concat() {
	assert_resolves(r#""a" + "b""#, string("ab")).await;
	assert_resolves(r#""a" + 1"#, string("a1")).await;
}

#[test]
async fn binary_comparison() {
	assert_resolves("1 === 1", boolean(true)).await;
	assert_resolves("1 < 0", boolean(false)).await;
}

#[test]
async fn call_object_freeze() {
	assert_resolves("Object.freeze({ a: 1 })", object([("a", num(1.0))])).await;
}

#[test]
async fn chain() {
	assert_resolves("({ a: 1 })?.a", num(1.0)).await;
	assert_resolves("[1, 2]?.[1]", num(2.0)).await;
}

#[test]
async fn conditional() {
	assert_resolves("true ? 1 : 2", num(1.0)).await;
	assert_resolves("0 ? 1 : 2", num(2.0)).await;
}

#[test]
async fn new_set() {
	assert_resolves(
		"new Set([2, 1, 2])",
		Plain(ConstantValue::Set([num(1.0), num(2.0)].into())),
	)
	.await;
}

#[test]
async fn object_literal() {
	assert_resolves(
		r#"{ a: 1, "b": "c", d: [true] }"#,
		object([
			("a", num(1.0)),
			("b", string("c")),
			("d", array([boolean(true)])),
		]),
	)
	.await;
}

#[test]
async fn object_numeric_key() {
	assert_resolves(
		"{ 1: 2 }",
		Plain(ConstantValue::Object(
			[(
				ConstantPropertyKey::Number(NotNan::new(1.0).unwrap()),
				num(2.0),
			)]
			.into(),
		)),
	)
	.await;
}

#[test]
async fn object_spread() {
	assert_resolves(
		"{ a: 1, ...{ b: 2 } }",
		object([("a", num(1.0)), ("b", num(2.0))]),
	)
	.await;
}

#[test]
async fn minified_booleans() {
	assert_resolves("!0", boolean(true)).await;
	assert_resolves("!1", boolean(false)).await;
}

#[test]
async fn unary() {
	assert_resolves("-1", num(-1.0)).await;
	assert_resolves("+\"2\"", num(2.0)).await;
	assert_resolves("~0", num(-1.0)).await;
	assert_resolves("typeof 1", string("number")).await;
}

#[test]
async fn computed_member() {
	assert_resolves("[1, 2][1]", num(2.0)).await;
	assert_resolves(r#"({ a: 1 })["a"]"#, num(1.0)).await;
}

#[test]
async fn static_member() {
	assert_resolves("({ a: 1 }).a", num(1.0)).await;
	assert_resolves("[1, 2, 3].length", num(3.0)).await;
}

#[test]
async fn static_member_of_identifier() {
	assert_resolves_in(
		"const y = { a: [1] }; const x = y.a;",
		array([num(1.0)]),
	)
	.await;
}

mod binary {
	use super::*;
	use macros::test;

	fn bigint(n: i64) -> Plain {
		Plain(ConstantValue::BigInt(n.into()))
	}

	#[test]
	async fn number_ops() {
		assert_resolves("7 % 3", num(1.0)).await;
		assert_resolves("(0 - 7) % 3", num(-1.0)).await;
		assert_resolves("5 % 0", Plain(ConstantValue::NaN)).await;
		assert_resolves("2 ** 10", num(1024.0)).await;
		assert_resolves("1 ** (0 / 0)", Plain(ConstantValue::NaN)).await;
		assert_resolves("1 ** (1 / 0)", Plain(ConstantValue::NaN)).await;
		assert_resolves("(0 / 0) ** 0", num(1.0)).await;
		assert_resolves("1 << 32", num(1.0)).await;
		assert_resolves("1 << 31", num(-2_147_483_648.0)).await;
		assert_resolves("(0 - 8) >> 1", num(-4.0)).await;
		assert_resolves("(0 - 1) >>> 0", num(4_294_967_295.0)).await;
		assert_resolves("(0 - 1) >>> 28", num(15.0)).await;
		assert_resolves("6 & 3", num(2.0)).await;
		assert_resolves("6 | 3", num(7.0)).await;
		assert_resolves("6 ^ 3", num(5.0)).await;
		assert_resolves("4294967296 | 0", num(0.0)).await;
	}

	#[test]
	async fn coercion() {
		assert_resolves(r#""3" * "4""#, num(12.0)).await;
		assert_resolves(r#""" - 1"#, num(-1.0)).await;
		assert_resolves(r#""0x10" - 0"#, num(16.0)).await;
		assert_resolves(r#""a" - 1"#, Plain(ConstantValue::NaN)).await;
		assert_resolves("true + true", num(2.0)).await;
		assert_resolves("null + 1", num(1.0)).await;
	}

	#[test]
	async fn string_concat() {
		assert_resolves(r#""" + 1.5"#, string("1.5")).await;
		assert_resolves(r#""" + 1e21"#, string("1e+21")).await;
		assert_resolves(r#""" + 0 / 0"#, string("NaN")).await;
		assert_resolves(r#""" + 1 / 0"#, string("Infinity")).await;
		assert_resolves(r#""a" + null"#, string("anull")).await;
		assert_resolves(r#""a" + true"#, string("atrue")).await;
		assert_resolves(r#""a" + 10n"#, string("a10")).await;
		assert_resolves(r#"1 + 2 + "3""#, string("33")).await;
		assert_resolves(r#""1" + 2 + 3"#, string("123")).await;
	}

	#[test]
	async fn bigint_ops() {
		assert_resolves("1n + 2n", bigint(3)).await;
		assert_resolves("7n / 2n", bigint(3)).await;
		assert_resolves("(0n - 7n) / 2n", bigint(-3)).await;
		assert_resolves("(0n - 7n) % 2n", bigint(-1)).await;
		assert_resolves("2n ** 10n", bigint(1024)).await;
		assert_resolves("1n << 4n", bigint(16)).await;
		assert_resolves("16n << (0n - 2n)", bigint(4)).await;
		assert_resolves("(0n - 5n) >> 1n", bigint(-3)).await;
		assert_resolves("(0n - 1n) >> 100000000000000000000n", bigint(-1))
			.await;
		assert_resolves("(0n - 6n) & 3n", bigint(2)).await;
		assert_resolves("6n | 3n", bigint(7)).await;
		assert_resolves("6n ^ 3n", bigint(5)).await;
	}

	#[test]
	async fn bigint_errors() {
		assert_errors("1n + 1").await;
		assert_errors("1n / 0n").await;
		assert_errors("1n % 0n").await;
		assert_errors("2n ** (0n - 1n)").await;
		assert_errors("2n ** 4294967295n").await;
		assert_errors("1n << 4294967295n").await;
		assert_errors("1n >>> 0n").await;
	}

	#[test]
	async fn strict_equality() {
		assert_resolves("1 === 1", boolean(true)).await;
		assert_resolves(r#"1 === "1""#, boolean(false)).await;
		assert_resolves("0 / 0 === 0 / 0", boolean(false)).await;
		assert_resolves("0 !== 0 / 0", boolean(true)).await;
		assert_resolves("null === null", boolean(true)).await;
		assert_resolves("1n === 1n", boolean(true)).await;
		assert_resolves("[] === 1", boolean(false)).await;
		assert_resolves("[] !== null", boolean(true)).await;
		assert_errors("[] === []").await;
	}

	#[test]
	async fn loose_equality() {
		assert_resolves(r#"1 == "1""#, boolean(true)).await;
		assert_resolves(r#""" == 0"#, boolean(true)).await;
		assert_resolves("true == 1", boolean(true)).await;
		assert_resolves(r#"true == "1""#, boolean(true)).await;
		assert_resolves("null == 0", boolean(false)).await;
		assert_resolves("null != 0", boolean(true)).await;
		assert_resolves("1n == 1", boolean(true)).await;
		assert_resolves("1n == 1.5", boolean(false)).await;
		assert_resolves(r#"1n == "1""#, boolean(true)).await;
		assert_resolves(r#"1n == "a""#, boolean(false)).await;
		assert_resolves("[] == null", boolean(false)).await;
		assert_errors("[] == 0").await;
		assert_errors("[] == []").await;
	}

	#[test]
	async fn relational() {
		assert_resolves("1 <= 1", boolean(true)).await;
		assert_resolves("2 >= 3", boolean(false)).await;
		assert_resolves("2 > 1", boolean(true)).await;
		assert_resolves("1 < 0 / 0", boolean(false)).await;
		assert_resolves("1 >= 0 / 0", boolean(false)).await;
		assert_resolves(r#""a" < "b""#, boolean(true)).await;
		assert_resolves(r#""10" < "9""#, boolean(true)).await;
		assert_resolves(r#""10" < 9"#, boolean(false)).await;
		// U+FF61 is a single UTF-16 code unit, which sorts after the
		// surrogate pair for U+1F600, unlike in UTF-8
		assert_resolves(r#""｡" < "😀""#, boolean(false)).await;
		assert_resolves("1n < 2", boolean(true)).await;
		assert_resolves("2n > 1.5", boolean(true)).await;
		assert_resolves("1n < 1 / 0", boolean(true)).await;
		assert_resolves("1n <= 0 / 0", boolean(false)).await;
		assert_resolves(r#"1n < "2""#, boolean(true)).await;
		assert_resolves(r#"1n < "a""#, boolean(false)).await;
		assert_resolves(r#"1n >= "a""#, boolean(false)).await;
		assert_errors("[] < 1").await;
	}

	#[test]
	async fn unsupported_operators() {
		assert_errors(r#""a" in []"#).await;
		assert_errors("1 instanceof []").await;
		assert_errors("[] + 1").await;
	}
}

mod unary {
	use super::*;
	use macros::test;

	#[test]
	async fn negation() {
		assert_resolves("-1.5", num(-1.5)).await;
		assert_resolves("-\"2\"", num(-2.0)).await;
		assert_resolves("-true", num(-1.0)).await;
		assert_resolves("-null", num(0.0)).await;
		assert_resolves("-\"a\"", Plain(ConstantValue::NaN)).await;
		assert_resolves("-5n", Plain(ConstantValue::BigInt((-5).into()))).await;
		assert_resolves("1 - -1", num(2.0)).await;
	}

	#[test]
	async fn plus() {
		assert_resolves("+\"0x10\"", num(16.0)).await;
		assert_resolves("+\"\"", num(0.0)).await;
		assert_resolves("+false", num(0.0)).await;
		assert_errors("+1n").await;
	}

	#[test]
	async fn bitwise_not() {
		assert_resolves("~5", num(-6.0)).await;
		assert_resolves("~4294967295", num(0.0)).await;
		assert_resolves("~\"1\"", num(-2.0)).await;
		assert_resolves("~5n", Plain(ConstantValue::BigInt((-6).into()))).await;
		assert_resolves("~-1n", Plain(ConstantValue::BigInt(0.into()))).await;
	}

	#[test]
	async fn logical_not() {
		assert_resolves("!\"\"", boolean(true)).await;
		assert_resolves("!\"a\"", boolean(false)).await;
		assert_resolves("!0n", boolean(true)).await;
		assert_resolves("![]", boolean(false)).await;
		assert_resolves("!!1", boolean(true)).await;
	}

	#[test]
	async fn type_of() {
		assert_resolves("typeof 0 / 0", Plain(ConstantValue::NaN)).await;
		assert_resolves("typeof (0 / 0)", string("number")).await;
		assert_resolves("typeof 1n", string("bigint")).await;
		assert_resolves("typeof !0", string("boolean")).await;
		assert_resolves("typeof \"\"", string("string")).await;
		assert_resolves("typeof void 0", string("undefined")).await;
		assert_resolves("typeof null", string("object")).await;
		assert_resolves("typeof []", string("object")).await;
		assert_resolves("typeof function() {}", string("function")).await;
		assert_resolves("typeof (() => 1)", string("function")).await;
		assert_resolves("typeof class {}", string("function")).await;
	}

	#[test]
	async fn void() {
		// the operand doesn't need to be constant
		assert_resolves("void (() => 1)", Plain(ConstantValue::Undefined))
			.await;
	}

	#[test]
	async fn errors() {
		assert_errors("-[]").await;
		assert_errors("~[]").await;
		assert_errors("delete 1").await;
	}
}

mod identifiers {
	use super::*;
	use macros::test;

	async fn assert_errors_in(body: &str) {
		if let Ok(v) = resolve_in(body).await {
			panic!(
				"expected `{body}` to fail to resolve, got {:?}",
				Plain::from(v)
			);
		}
	}

	#[test]
	async fn in_expressions() {
		assert_resolves_in("const y = 2; const x = y * y + 1;", num(5.0)).await;
		assert_resolves_in(
			"const a = \"a\", b = a + \"b\"; const x = b + a;",
			string("aba"),
		)
		.await;
	}

	#[test]
	async fn var_and_function_scope() {
		assert_resolves_in(
			"var y = 1; function f() { const x = y; }",
			num(1.0),
		)
		.await;
	}

	#[test]
	async fn shadowed_globals() {
		assert_resolves_in(
			"const undefined = 1; const x = undefined;",
			num(1.0),
		)
		.await;
		assert_resolves_in("const NaN = \"a\"; const x = NaN;", string("a"))
			.await;
	}

	#[test]
	async fn unknown_global_errors() {
		assert_errors("window").await;
	}

	#[test]
	async fn uninitialized_errors() {
		assert_errors_in("let y; const x = y;").await;
	}

	#[test]
	async fn compound_assignment_errors() {
		assert_errors_in("let y = 1; y += 1; const x = y;").await;
		assert_errors_in("let y = 1; y++; const x = y;").await;
	}

	#[test]
	async fn destructured_errors() {
		assert_errors_in("const { a: y } = { a: 1 }; const x = y;").await;
		assert_errors_in("const [y] = [1]; const x = y;").await;
		assert_errors_in("let y; [y] = [1]; const x = y;").await;
	}

	#[test]
	async fn non_variable_errors() {
		assert_errors_in("function y() {} const x = y;").await;
		assert_errors_in("function f(y) { const x = y; }").await;
	}

	#[test]
	async fn cycles_error() {
		assert_errors_in("let y; y = y + 1; const x = y;").await;
		assert_errors_in("let a, b; a = b; b = a; const x = a;").await;
		assert_errors_in("let a, b; a = [b]; b = [a]; const x = a;").await;
	}
}

mod static_member {
	use super::*;
	use macros::test;

	async fn assert_resolves_with(
		body: &str,
		modules: &[(u32, &str)],
		expected: Plain,
	) {
		match resolve_in_modules(body, modules).await {
			Ok(v) => assert_eq!(Plain::from(v), expected, "for `{body}`"),
			Err(e) => panic!("failed to resolve in `{body}`: {e:?}"),
		}
	}

	async fn assert_errors_with(body: &str, modules: &[(u32, &str)]) {
		if let Ok(v) = resolve_in_modules(body, modules).await {
			panic!(
				"expected `{body}` to fail to resolve, got {:?}",
				Plain::from(v)
			);
		}
	}

	const WREQ_D: (u32, &str) =
		(2, "n.d(t, { A: () => r, S: () => s }); const r = 5, s = \"abc\";");

	#[test]
	async fn wreq_d_export() {
		assert_resolves_with("const x = n(2).A;", &[WREQ_D], num(5.0)).await;
	}

	#[test]
	async fn export_of_imported_module() {
		assert_resolves_with("var r = n(2); const x = r.A;", &[WREQ_D], num(5.0))
			.await;
	}

	#[test]
	async fn exports_assignment() {
		assert_resolves_with(
			"const x = n(2).foo;",
			&[(2, "t.foo = \"x\";")],
			string("x"),
		)
		.await;
		assert_resolves_with(
			"const x = n(2).a;",
			&[(2, "e.exports = { a: 1 };")],
			num(1.0),
		)
		.await;
	}

	#[test]
	async fn nested_export() {
		assert_resolves_with(
			"const x = n(2).A.b;",
			&[(2, "n.d(t, { A: () => o }); const o = { b: 1 };")],
			num(1.0),
		)
		.await;
	}

	#[test]
	async fn re_export() {
		assert_resolves_with(
			"const x = n(2).A;",
			&[
				(2, "n.d(t, { A: () => r.B }); var r = n(3);"),
				(3, "n.d(t, { B: () => b }); const b = true;"),
			],
			boolean(true),
		)
		.await;
	}

	#[test]
	async fn property_of_export() {
		assert_resolves_with("const x = n(2).S.length;", &[WREQ_D], num(3.0))
			.await;
	}

	#[test]
	async fn length() {
		assert_resolves("[1, 2, 3].length", num(3.0)).await;
		assert_resolves("\"abc\".length", num(3.0)).await;
		// JS strings are UTF-16
		assert_resolves("\"\u{1F600}\".length", num(2.0)).await;
		assert_resolves_in(
			"const y = [[1], 2]; const x = y.length;",
			num(2.0),
		)
		.await;
	}

	#[test]
	async fn export_errors() {
		// missing export
		assert_errors_with("const x = n(2).B;", &[WREQ_D]).await;
		// module not in the cache
		assert_errors_with("const x = n(4).A;", &[WREQ_D]).await;
		// an object export with no node for the whole object
		assert_errors_with(
			"const x = n(2).A;",
			&[(2, "n.d(t, { A: () => o }); const o = { b: 1 };")],
		)
		.await;
		// functions
		assert_errors_with(
			"const x = n(2).f;",
			&[(2, "n.d(t, { f: () => f }); function f() {}")],
		)
		.await;
	}

	#[test]
	async fn export_cycle_errors() {
		assert_errors_with(
			"const x = n(2).A;",
			&[
				(2, "n.d(t, { A: () => r.B }); var r = n(3);"),
				(3, "n.d(t, { B: () => r.A }); var r = n(2);"),
			],
		)
		.await;
	}

	#[test]
	async fn property_errors() {
		assert_errors("null.a").await;
		assert_errors("undefined.a").await;
		assert_errors("(1).toFixed").await;
		assert_errors("[].map").await;
		assert_errors("\"a\".at").await;
	}
}

mod objects {
	use super::*;
	use macros::test;

	/// An object with keys that may be numbers
	fn keyed(
		props: impl IntoIterator<Item = (ConstantPropertyKey, Plain)>,
	) -> Plain {
		Plain(ConstantValue::Object(props.into_iter().collect()))
	}
	fn n_key(n: f64) -> ConstantPropertyKey {
		ConstantPropertyKey::Number(NotNan::new(n).unwrap())
	}
	fn s_key(s: &str) -> ConstantPropertyKey {
		ConstantPropertyKey::String(s.to_owned())
	}

	#[test]
	async fn numeric_keys() {
		assert_resolves(r#"{ "1": 2 }"#, keyed([(n_key(1.0), num(2.0))])).await;
		assert_resolves(
			r#"{ 1: "a", "1": "b" }"#,
			keyed([(n_key(1.0), string("b"))]),
		)
		.await;
		assert_resolves("{ 1.50: 1 }", keyed([(n_key(1.5), num(1.0))])).await;
		assert_resolves("{ 0x10: 1 }", keyed([(n_key(16.0), num(1.0))])).await;
		// not canonical numbers
		assert_resolves(r#"{ "01": 1 }"#, keyed([(s_key("01"), num(1.0))]))
			.await;
		assert_resolves(r#"{ "-0": 1 }"#, keyed([(s_key("-0"), num(1.0))]))
			.await;
		assert_resolves(r#"{ "": 1 }"#, keyed([(s_key(""), num(1.0))])).await;
	}

	#[test]
	async fn computed_keys() {
		assert_resolves(r#"{ ["a" + "b"]: 1 }"#, object([("ab", num(1.0))]))
			.await;
		assert_resolves("{ [1 + 1]: 1 }", keyed([(n_key(2.0), num(1.0))]))
			.await;
		assert_resolves("{ [[1, 2]]: 1 }", object([("1,2", num(1.0))])).await;
		assert_resolves("{ [true]: 1 }", object([("true", num(1.0))])).await;
		assert_resolves(
			r#"{ ["__proto__"]: 1 }"#,
			object([("__proto__", num(1.0))]),
		)
		.await;
	}

	#[test]
	async fn shorthand() {
		assert_resolves_in(
			"const a = 1; const x = { a };",
			object([("a", num(1.0))]),
		)
		.await;
		assert_resolves_in(
			"const __proto__ = 1; const x = { __proto__ };",
			object([("__proto__", num(1.0))]),
		)
		.await;
	}

	#[test]
	async fn later_properties_win() {
		assert_resolves("{ a: 1, a: 2 }", object([("a", num(2.0))])).await;
		assert_resolves("{ a: 1, ...{ a: 2 } }", object([("a", num(2.0))]))
			.await;
		assert_resolves("{ ...{ a: 2 }, a: 1 }", object([("a", num(1.0))]))
			.await;
	}

	#[test]
	async fn spreads() {
		assert_resolves(
			"{ ...[1, 2] }",
			keyed([(n_key(0.0), num(1.0)), (n_key(1.0), num(2.0))]),
		)
		.await;
		assert_resolves(
			r#"{ ..."ab" }"#,
			keyed([(n_key(0.0), string("a")), (n_key(1.0), string("b"))]),
		)
		.await;
		assert_resolves(
			"{ ...null, ...undefined, ...1, ...true, a: 1 }",
			object([("a", num(1.0))]),
		)
		.await;
		assert_resolves_in(
			"const y = { a: [1] }; const x = { ...y, b: 2 };",
			object([("a", array([num(1.0)])), ("b", num(2.0))]),
		)
		.await;
	}

	#[test]
	async fn nested() {
		assert_resolves(
			"{ a: { b: [{}] } }",
			object([("a", object([("b", array([object([])]))]))]),
		)
		.await;
	}

	#[test]
	async fn properties() {
		assert_resolves("({ a: 1 }).a", num(1.0)).await;
		assert_resolves("({ a: 1 }).b", Plain(ConstantValue::Undefined)).await;
		assert_resolves("({ Infinity: 1 }).Infinity", num(1.0)).await;
		assert_resolves("({ a: { b: 2 } }).a.b", num(2.0)).await;
	}

	#[test]
	async fn errors() {
		assert_errors("{ __proto__: null }").await;
		assert_errors(r#"{ "__proto__": null }"#).await;
		assert_errors("{ get a() { return 1; } }").await;
		assert_errors("{ set a(v) {} }").await;
		assert_errors("{ a() {} }").await;
		assert_errors("{ [window]: 1 }").await;
		assert_errors(r#"{ ..."\u{1F600}" }"#).await;
		assert_errors("({}).toString").await;
	}
}
