use super::*;
use macros::test;

fn write_once(body: &str) -> bool {
	let alloc = Allocator::new();
	let source = format!("0,function(e, t, n) {{ {body} }}");
	let p = WebpackAstParser::try_new(&alloc, &source).unwrap();
	let scoping = p.sema().scoping();
	let syms = scoping
		.symbol_ids()
		.filter(|&id| scoping.symbol_name(id) == "x")
		.collect_vec();
	if syms.len() != 1 {
		panic!("expected exactly one symbol named `x`, found {syms:?}");
	}
	p.is_write_once(&syms[0])
}

fn not_write_once(body: &str) -> bool {
	!write_once(body)
}

#[test]
fn const_decl() {
	assert!(write_once("const x = 1;"));
}

#[test]
fn let_uninit_then_assigned_once() {
	assert!(write_once("let x; x = 1;"));
}

#[test]
fn var_uninit_then_assigned_once() {
	assert!(write_once("var x; x = 1;"));
}

#[test]
fn let_never_written() {
	assert!(write_once("let x;"));
}

#[test]
fn let_init_never_reassigned() {
	assert!(write_once("let x = 1; console.log(x);"));
}

#[test]
fn let_reads_do_not_count() {
	assert!(write_once("let x; x = 1; console.log(x, x + 1);"));
}

#[test]
fn let_assigned_twice() {
	assert!(not_write_once("let x; x = 1; x = 2;"));
}

#[test]
fn var_init_then_reassigned() {
	assert!(not_write_once("var x = 1; x = 2;"));
}

#[test]
fn let_init_then_updated() {
	assert!(not_write_once("let x = 1; x++;"));
}

#[test]
fn let_init_then_compound_assigned() {
	assert!(not_write_once("let x = 1; x += 1;"));
}

#[test]
fn function_redeclared() {
	assert!(not_write_once(
		"function x() { return 1; } function x() { return 2; }"
	));
}
