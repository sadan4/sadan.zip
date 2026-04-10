#![allow(clippy::needless_raw_string_hashes)]
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
