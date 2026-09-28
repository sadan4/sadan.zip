use super::*;
use macros::test;

#[test]
fn collects_intl_keys_with_spans() {
	let p = parse!("test_data/wp/wreq.d/objectExport.js");
	let keys = p.dbg_intl_keys();
	assert_debug_snapshot!(keys);
}

#[test]
/// `intl.string(foo ? intlMod.t.key1 : intlMod.t.key2)`
fn collects_keys_across_accessors_and_ternaries() {
	let p = parse!("test_data/wp/finds/intlKeys2.js");
	let keys = p.dbg_intl_keys();
	assert_debug_snapshot!(keys);
}
