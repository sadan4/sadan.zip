use super::*;
use macros::test;

#[test]
fn detects_create_apex_experiment() {
	let p = parse!("test_data/wp/experiments/createApexExperiment.js");
	let res = p.is_create_apex_experiment_module();
	assert_debug_snapshot!(res, @r#"
	Some(
	    Named(
	        "Ay",
	    ),
	)
	"#);
}

/// module `36200` from build `c5c3a0d847070a820cddbbb4e4d85b47934ab240`
#[test]
fn detects_create_experiment() {
	let p = parse!("test_data/wp/experiments/createExperiment.js");
	let res = p.is_create_experiment_module();
	assert_debug_snapshot!(res, @r#"
	Some(
	    Named(
	        "A",
	    ),
	)
	"#);
}
