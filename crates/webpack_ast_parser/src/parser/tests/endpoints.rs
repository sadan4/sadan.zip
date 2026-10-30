use insta::assert_debug_snapshot;

#[test]
fn parses_endpoints() {
	let p = parse!("test_data/wp/wreq.d/endpoints.js");
	let endpoints = p
		.as_endpoints_module()
		.unwrap()
		.unwrap();
	assert_debug_snapshot!(endpoints);
}
