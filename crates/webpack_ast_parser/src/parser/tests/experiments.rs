use super::*;
use explorer_types::experiments::{
	ApexExperiment,
	ExperimentKind,
	ExperimentScope,
	NormalExperiment,
	Treatment,
	Variation,
};
use macros::test;
use serde_json::json;

/// module `521169` from build `aa264a149d444461154df6f2142270838dbad6e1`
///
/// `createExperiment` is `n(945810).mj`
#[test]
fn finds_user_experiment() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments =
		p.get_defined_apex_experiments(945810.into(), "mj".into());
	let [Experiment { loc, obj }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	assert_eq!(
		obj,
		&ExperimentKind::Apex(ApexExperiment {
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
			kind: ExperimentScope::User,
		})
	);
	assert_eq!(loc.id, p.get_module_id().unwrap().id);
}

#[test]
fn ignores_other_exports_of_create_experiment_module() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments =
		p.get_defined_apex_experiments(945810.into(), "notMj".into());
	assert_eq!(experiments, vec![]);
}

#[test]
fn ignores_other_modules() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments =
		p.get_defined_apex_experiments(111111.into(), "mj".into());
	assert_eq!(experiments, vec![]);
}

#[test]
fn apex_ignores_normal_experiments() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments = p.get_defined_apex_experiments(600975.into(), "C".into());
	assert_eq!(experiments, vec![]);
}

/// module `402313` from build `6d330cc72605bec8b66e953e75d47efa2069d945`
///
/// `createExperiment` is `i(600975).C`
#[test]
fn finds_guild_experiment() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments =
		p.get_defined_normal_experiments(600975.into(), "C".into());
	let [Experiment { loc, obj }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	assert_eq!(
		obj,
		&ExperimentKind::Normal(NormalExperiment {
			kind: ExperimentScope::Guild,
			id: "2026-04_voice_user_duration".to_owned(),
			label: "Voice User Duration".to_owned(),
			default_config: json!({ "enabled": false }),
			treatments: vec![Treatment {
				id: 1,
				label: "Allow guild members to see each others duration in \
				        the channel list"
					.to_owned(),
				config: json!({ "enabled": true }),
			}],
			common_trigger_point: None,
		})
	);
	assert_eq!(loc.id, p.get_module_id().unwrap().id);
}

#[test]
fn normal_ignores_other_exports_of_create_experiment_module() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments =
		p.get_defined_normal_experiments(600975.into(), "notC".into());
	assert_eq!(experiments, vec![]);
}

#[test]
fn normal_ignores_other_modules() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments =
		p.get_defined_normal_experiments(111111.into(), "C".into());
	assert_eq!(experiments, vec![]);
}

#[test]
fn normal_ignores_apex_experiments() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments =
		p.get_defined_normal_experiments(945810.into(), "mj".into());
	assert_eq!(experiments, vec![]);
}
