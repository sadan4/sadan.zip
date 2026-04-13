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

#[test]
fn break_and_continue() {
	let source = "for(var i in set)if(i%2===0)break;else continue;
function foo(){while(1){if (a)continue;test();}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	for (var i in set)
	  if (i % 2 === 0)
	    break;
	  else
	    continue;
	function foo() {
	  while (1) {
	    if (a)
	      continue;
	    test();
	  }
	}
	");
}

#[test]
fn null_keyword() {
	let source = "1||null;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"1 || null;");
}

#[test]
fn exponential_operators() {
	let source = "2**3;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"2 ** 3;");
}

#[test]
fn for_loops() {
	let source = "for(var value of map)if (value.length%3===0)console.log(value);for(var key in myMap)print(key);for(var value of myMap)print(value);";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	for (var value of map)
	  if (value.length % 3 === 0)
	    console.log(value);
	for (var key in myMap)
	  print(key);
	for (var value of myMap)
	  print(value);
	");
}

#[test]
fn chained_and_nested_if_statements() {
	let source = "if(a%7===0)b=1;else if(a%9===1) b =  2;else if(a%5===3){b=a/2;b++;} else b= 3;{if (a>b){a();pretty();}else if (a+b)e();reset();}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	if (a % 7 === 0)
	  b = 1;
	else if (a % 9 === 1)
	  b = 2;
	else if (a % 5 === 3) {
	  b = a / 2;
	  b++;
	} else
	  b = 3;
	{
	  if (a > b) {
	    a();
	    pretty();
	  } else if (a + b)
	    e();
	  reset();
	}
	");
}

#[test]
fn try_catch_statements() {
	let source = "try{a(b());}catch(e){f()}finally{f();}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	try {
	  a(b());
	} catch (e) {
	  f()
	} finally {
	  f();
	}
	");
}

#[test]
fn object_spreads() {
	let source = "const a = {a:4,...{a: 5,b: 42}};";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	const a = {
	  a: 4,
	  ...{
	    a: 5,
	    b: 42
	  }
	};
	");
}

#[test]
fn object_destructuring() {
	let source = "let{x,y}=getXYFromTouchOrPointer(e);var test = function({x,y}){foo(x,y);}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	let {x, y} = getXYFromTouchOrPointer(e);
	var test = function({x, y}) {
	  foo(x, y);
	}
	");
}

#[test]
fn declaration_for_dollar_sign() {
	let source = "let $=1;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"let $ = 1;");
}


#[test]
fn declaration_for_underscore() {
	let source = "let _=1;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"let _ = 1;");
}

#[test]
fn declaration_for_unicode_name() {
	let source = "let \u{00e1}=1;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"let \u{00e1} = 1;");
}

#[test]
fn const_declaration_for_unicode_name() {
	let source = "const \u{00e1}=1;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"const \u{00e1} = 1;");
}

#[test]
fn yield_number() {
	let source = "function *one() {yield 1;}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	function *one() {
	  yield 1;
	}
	");
}

#[test]
fn object_expressions() {
	let source = r#"var mapping={original:[1,2,3],formatted:[],count:0};var obj={'foo':1,bar:"2",cat:{dog:'1989'}}"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"
	var mapping = {
	  original: [1, 2, 3],
	  formatted: [],
	  count: 0
	};
	var obj = {
	  'foo': 1,
	  bar: "2",
	  cat: {
	    dog: '1989'
	  }
	}
	"#);
}

#[test]
fn block_statements() {
	let source = "{ print(1); print(2); }";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	{
	  print(1);
	  print(2);
	}
	");
}

#[test]
fn assignment_expressions() {
	let source = "var exp='a string';c=+a+(0>a?b:0);c=(1);var a=(1);";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	var exp = 'a string';
	c = +a + (0 > a ? b : 0);
	c = (1);
	var a = (1);
	");
}

#[test]
fn with_statements() {
	let source = "with(obj)log('first');with(nice){log(1);log(2);}done();";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	with (obj)
	  log('first');
	with (nice) {
	  log(1);
	  log(2);
	}
	done();
	");
}

#[test]
fn switch_statements() {
	let source = r#"switch (a) { case 1, 3: log("odd");break;case 2:log("even");break;case 42:case 89: log(a);default:log("interesting");log(a);}log("done");"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"
	switch (a) {
	case 1, 3:
	  log("odd");
	  break;
	case 2:
	  log("even");
	  break;
	case 42:
	case 89:
	  log(a);
	default:
	  log("interesting");
	  log(a);
	}
	log("done");
	"#);
}

#[test]
fn generator_expressions() {
	let source = "function *max(){var a=yield;var b=yield 10;if(a>b)return a;else return b;}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	function *max() {
	  var a = yield;
	  var b = yield 10;
	  if (a > b)
	    return a;
	  else
	    return b;
	}
	");
}

#[test]
fn block_comments() {
	let source = r#"/** this
 * is
 * block
 * comment
 */
var a = 10;"#;
	let out = format2(source).unwrap();
	assert_eq!(out, source);
}
#[test]
fn let_assignments() {
	let source = "for(var i=0;i<names.length;++i){let name=names[i];let person=persons[i];}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	for (var i = 0; i < names.length; ++i) {
	  let name = names[i];
	  let person = persons[i];
	}
	");
}

#[test]
fn anonymous_functions() {
	let source = "setTimeout(function(){alert(1);},2000); function test(arg){console.log(arg);}test(a=>a+2);
var onClick = function() { console.log('click!'); };console.log('done');var onStart = function() { a(); }, onFinish = function() { b(); };
var onStart = function() {}, delay=1000, belay=document.activeElement;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	setTimeout(function() {
	  alert(1);
	}, 2000);
	function test(arg) {
	  console.log(arg);
	}
	test(a => a + 2);
	var onClick = function() {
	  console.log('click!');
	};
	console.log('done');
	var onStart = function() {
	  a();
	}
	  , onFinish = function() {
	  b();
	};
	var onStart = function() {}
	  , delay = 1000
	  , belay = document.activeElement;
	");
	
}

#[test]
fn arrow_functions() {
	let source1 = "const double=x=>x*2;";
	let source2 = "const sum=(a,b)=>a+b;";
	let source3 = "const double=x=>{return x*2;}";
	let source4 = "const sum=(a,b,c)=>{const val=a+b+c;return val;}";
	let out1 = format2(source1).unwrap();
	let out2 = format2(source2).unwrap();
	let out3 = format2(source3).unwrap();
	let out4 = format2(source4).unwrap();
	assert_snapshot!(out1, @"const double = x => x * 2;");
	assert_snapshot!(out2, @"const sum = (a, b) => a + b;");
	assert_snapshot!(out3, @"
	const double = x => {
	  return x * 2;
	}
	");
	assert_snapshot!(out4, @"
	const sum = (a, b, c) => {
	  const val = a + b + c;
	  return val;
	}
	");
}

mod files_with_comments;

#[test]
fn methods_on_literals() {
	let source = r#"num=1 .toString();str="abc" . toUpperCase();"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"
	num = 1 .toString();
	str = "abc".toUpperCase();
	"#);
}
