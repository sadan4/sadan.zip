use super::test;

#[test]
fn get_num() {
	let p = parse!("test_data/wp/concatenated_module.js");
	let num = p.num_concatenated_modules();
	assert_eq!(num, 11);
}

#[test]
fn gets_num_for_non_concatenated_module() {
	let p = parse!("test_data/wp/module.js");
	let num = p.num_concatenated_modules();
	assert_eq!(num, 1);
}
