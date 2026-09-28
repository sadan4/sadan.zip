use super::*;
use macros::test;

#[test]
fn parses_module_id() {
	let p = parse!("test_data/wp/module.js");
	let id = p.get_module_id();

	assert_eq!(id.unwrap().id, ModuleId(317269));
}

#[test]
fn fails_to_parse_malformed_module_id() {
	let p = parse!("test_data/wp/bad/badModule1.js");
	let id = p.get_module_id();
	assert!(id.is_err());
}

#[test]
fn fails_to_parse_missing_module_id() {
	let p = parse!("test_data/wp/bad/badModule2.js");
	let id = p.get_module_id();
	assert!(id.is_err());
}

#[test]
fn gets_ids_of_dependencies() {
	let parser = parse!("test_data/wp/concatenated_module.js");
	let (sync, lazy) = parser.dbg_outgoing_deps();
	assert_debug_snapshot!("sync_deps", sync);
	assert_debug_snapshot!("lazy_deps", lazy);
}
