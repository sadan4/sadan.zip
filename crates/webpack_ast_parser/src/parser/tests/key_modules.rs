use super::*;
use macros::test;

#[test]
fn detects_create_experiment() {
	let p = parse!("test_data/wp/wreq.d/createExperiment.js");
	let res = p.is_create_experiment_module();
	assert_debug_snapshot!(res, @r#"
	Some(
	    Named(
	        "Ay",
	    ),
	)
	"#);
}
