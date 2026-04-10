#![allow(clippy::needless_raw_string_hashes)]
//! ported from <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/entrypoints/formatter_worker/JavaScriptFormatter.test.ts>
use insta::assert_snapshot;
use macros::test;
use pretty_printer::{format, format2};
#[test]
fn await_expressions() {
	let source = r#"(async () => { await someFunctionThatNeedsAwaiting(); callSomeOtherFunction(); })();"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"(async () => {\n  await someFunctionThatNeedsAwaiting();\n  callSomeOtherFunction();\n}\n)();\n");
}

#[test]
fn async_function_expressions() {
	let source = "async function foo() {return await Promise.resolve(1);}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	async function foo() {
	  return await Promise.resolve(1);
	}
	");
}

#[test]
fn top_level_await() {
	let source = "const myFile=await import(\n\"my-file.mjs\");";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"const myFile = await import("my-file.mjs");"#);
}

#[test]
fn idents_with_escaped_characters() {
	let source = r#"const x=42;let \u0275_escaped;"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r"
	const x = 42;
	let \u0275_escaped;
	");
}
