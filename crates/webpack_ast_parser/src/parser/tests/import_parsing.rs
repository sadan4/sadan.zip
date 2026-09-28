use super::*;
use macros::test;

fn k(s: &'static str) -> ExportMapKey {
	SmolStr::new_static(s).into()
}

#[test]
fn only_reexported_export() {
	let p = parse!("test_data/wp/imports/reExport.js");
	let uses = p.dbg_uses_of_import(ModuleId(999001), &[k("foo")]);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[5:21->5:24) foo",
	]
	"#);
}
#[test]
fn reexport_with_other_uses() {
	let p = parse!("test_data/wp/imports/reExport.js");
	let uses = p.dbg_uses_of_import(ModuleId(999001), &[k("bar")]);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[6:22->6:25) bar",
	    "[10:18->10:21) bar",
	]
	"#);
}
#[test]
fn empty_when_no_uses() {
	let p = parse!("test_data/wp/imports/reExport.js");
	let uses = p.dbg_uses_of_import(ModuleId(999001), &[k("baz")]);
	assert_debug_snapshot!(uses, @"[]");
}
#[test]
fn empty_when_not_imported() {
	let p = parse!("test_data/wp/imports/reExport.js");
	let uses = p.dbg_uses_of_import(ModuleId(999003), &[k("foo")]);
	assert_debug_snapshot!(uses, @"[]");
}

#[test]
fn empty_when_no_uses_2() {
	let p = parse!("test_data/wp/imports/indirectCall.js");
	let uses = p.dbg_uses_of_import(ModuleId(999002), &[k("bar")]);
	assert_debug_snapshot!(uses, @"[]");
}
#[test]
fn empty_when_not_imported_2() {
	let p = parse!("test_data/wp/imports/indirectCall.js");
	let uses = p.dbg_uses_of_import(ModuleId(999004), &[k("foo")]);
	assert_debug_snapshot!(uses, @"[]");
}
#[test]
fn indirect_call() {
	let p = parse!("test_data/wp/imports/indirectCall.js");
	let uses = p.dbg_uses_of_import(ModuleId(999002), &[k("foo")]);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[9:22->9:25) foo",
	]
	"#);
}
#[test]
fn direct_call() {
	let p = parse!("test_data/wp/imports/directCall.js");
	let uses = p.dbg_uses_of_import(ModuleId(999003), &[k("foo3")]);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[8:29->8:33) foo3",
	]
	"#);
}

#[test]
fn none_when_wreq_unused() {
	let p = parse!("test_data/wp/imports/directCall.js");
	let uses = p.get_uses_of_import(ModuleId(0), &[]);
	assert_eq!(uses, vec![]);
}

#[test]
fn node_default_exports() {
	let p = parse!("test_data/wp/imports/nodeModule.js");
	let uses = p.dbg_uses_of_import(ModuleId(999005), &[ExportMapKey::Default]);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[15:8->15:12) _1()",
	    "[20:15->20:19) _1()",
	]
	"#);
}

#[test]
fn node_named_exports() {
	let p = parse!("test_data/wp/imports/nodeModule.js");
	let uses = p.dbg_uses_of_import(ModuleId(999005), &[k("qux")]);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[16:20->16:23) qux",
	    "[19:13->19:16) qux",
	]
	"#);
}

#[test]
#[ignore = "TODO"]
fn wreq_t_use() {
	let p = parse!("test_data/wp/imports/wreqTWrapper.js");
	let _uses = p.dbg_uses_of_import(582128.into(), &[]);
}

#[test]
fn saved_direct_call_in_assign() {
	let p = parse!("test_data/wp/imports/savedDirectCall.js");
	let uses = p.dbg_uses_of_import(
		ModuleId(891600),
		&[
			ExportMapKey::Named(SmolStr::new_static("A")),
			ExportMapKey::Named(SmolStr::new_static("reactParserFor")),
		],
	);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[10:28->10:42) reactParserFor",
	]
	"#);
}

#[test]
fn saved_direct_call() {
	let p = parse!("test_data/wp/imports/savedDirectCall.js");
	let uses = p.dbg_uses_of_import(
		ModuleId(891600),
		&[
			ExportMapKey::Named(SmolStr::new_static("A")),
			ExportMapKey::Named(SmolStr::new_static("astParserFor")),
		],
	);
	assert_debug_snapshot!(uses, @r#"
	[
	    "[11:12->11:24) astParserFor",
	]
	"#);
}
