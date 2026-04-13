use super::{test, *};

#[test]
fn simple_template_literal() {
	let source = "var foo = `bar`;";
	let out = format2(source).unwrap();
	assert_eq!(source, out);
}

#[test]
fn multiline_template() {
	let source = "var foo = `this
bar`;";
	let out = format2(source).unwrap();
	assert_eq!(source, out);
}

#[test]
fn string_substitution() {
	let source = r#"var a=`I have ${credit+cash}$`;
var a=`${name} has ${credit+cash}${currency?currency:"$"}`;"#;
	let out = format2(source).unwrap();
	assert_snapshot!(out, @r#"
	var a = `I have ${credit + cash}$`;
	var a = `${name} has ${credit + cash}${currency ? currency : "$"}`;
	"#);
}

#[test]
fn tagged_templates() {
	// lol, typo in original test case
	let source = "escapeHtml`<div class=${classnName} width=${a+b}/>`;";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"escapeHtml`<div class=${classnName} width=${a + b}/>`;");
}
