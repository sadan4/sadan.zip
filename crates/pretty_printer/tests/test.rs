#![allow(clippy::needless_raw_string_hashes)]
//! ported from <https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/entrypoints/formatter_worker/JavaScriptFormatter.test.ts>
use insta::assert_snapshot;
use macros::test;
use pretty_printer::{format, format2};
#[test]
fn await_expressions() {
	let source = r#"(async () => { await someFunctionThatNeedsAwaiting(); callSomeOtherFunction(); })();"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	(async () => {
	  await someFunctionThatNeedsAwaiting();
	  callSomeOtherFunction();
	}
	)();
	");
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

#[test]
fn nullish_coalescing() {
	let source = "false??true";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"false ?? true");
}

#[test]
fn optional_chaining() {
	let source = "var x=a?.b;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"var x = a?.b;");
}

#[test]
fn logical_assignment() {
	let source = "x||=1;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"x ||= 1;");
}

#[test]
fn numeric_separators() {
	let source = "x=1_000;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"x = 1_000;");
}

#[test]
fn do_while_loops() {
	let source = r#"
	function demo() {
	do {} while (false);
	if (true) {}
	}
	function demo() {do {} while (false);if (true) {}}
	"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	function demo() {
	  do {} while (false);
	  if (true) {}
	}
	function demo() {
	  do {} while (false);
	  if (true) {}
	}
	");
}

#[test]
fn while_loops() {
	let source = "while(true){print('infinity');}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	while (true) {
	  print('infinity');
	}
	");
}

#[test]
fn function_statements() {
	let source = "function test(a,b,c){a*=b;return c+a;}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	function test(a, b, c) {
	  a *= b;
	  return c + a;
	}
	");
}

#[test]
fn variable_statements() {
	let source =
		r#"var a=1,b={},c=2,d="hello world";var a,b,c,d=2,e,f=3;var a={};"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"
	var a = 1
	  , b = {}
	  , c = 2
	  , d = "hello world";
	var a, b, c, d = 2, e, f = 3;
	var a = {};
	"#);
}

#[test]
fn array_literals() {
	let source = "var arr=[3,2,1,0]";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"var arr = [3, 2, 1, 0]");
}

#[test]
fn ternary_expressions() {
	let source = "a>b?a:b";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"a > b ? a : b");
}

#[test]
fn labeled_statements() {
	let source = "firstLoop:while(true){break firstLoop;continue firstLoop;}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	firstLoop: while (true) {
	  break firstLoop;
	  continue firstLoop;
	}
	");
}

#[test]
fn multiple_statements_on_one_line() {
	let source = "rebuild(),show(),hasNew?refresh():noop();";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	rebuild(),
	show(),
	hasNew ? refresh() : noop();
	");
}

#[test]
fn if_statements() {
	let source = "if(a<b)log(a);else log(b);if(a<b){log(a)}else{log(b);}if(a===b)log('equals');if(a!==b){log('non-eq');}if(a>b&&b>c){print(a);print(b);}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	if (a < b)
	  log(a);
	else
	  log(b);
	if (a < b) {
	  log(a)
	} else {
	  log(b);
	}
	if (a === b)
	  log('equals');
	if (a !== b) {
	  log('non-eq');
	}
	if (a > b && b > c) {
	  print(a);
	  print(b);
	}
	");
}





// TODO: pick fix
#[test]
#[ignore = "TODO"]
fn methods_on_literals() {
	let source = r#"num=1 .toString();str="abc" . toUpperCase();"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"
	num = 1 .toString();
	str = "abc".toUpperCase();
	"#);
}
