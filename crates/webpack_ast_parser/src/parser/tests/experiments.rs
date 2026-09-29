use super::*;
use explorer_types::experiments::{ExperimentKind, UserExperiment, Variation};
use macros::test;
use serde_json::json;

/// module `521169` from build `aa264a149d444461154df6f2142270838dbad6e1`
///
/// `createExperiment` is `n(945810).mj`
#[test]
fn finds_user_experiment() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p.get_defined_experiments(945810.into(), "mj".into());
	let [Experiment { loc, obj }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	assert_eq!(
		obj,
		&ExperimentKind::User(UserExperiment {
			name: "2026-08-tiny-bronco".to_owned(),
			default_config: json!({ "enabled": false }),
			label: None,
			variations: vec![
				Variation {
					key: "0".to_owned(),
					config: json!({ "enabled": false }),
				},
				Variation {
					key: "1".to_owned(),
					config: json!({ "enabled": true }),
				},
			],
			treatments: vec![],
			common_trigger_point: None,
		})
	);
	assert_eq!(loc.id, p.get_module_id().unwrap().id);
}

#[test]
fn ignores_other_exports_of_create_experiment_module() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p.get_defined_experiments(945810.into(), "notMj".into());
	assert_eq!(experiments, vec![]);
}

#[test]
fn ignores_other_modules() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p.get_defined_experiments(111111.into(), "mj".into());
	assert_eq!(experiments, vec![]);
}
