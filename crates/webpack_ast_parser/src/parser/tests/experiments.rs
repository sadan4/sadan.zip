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
use serde_json::{Value, json};

/// module `521169` from build `aa264a149d444461154df6f2142270838dbad6e1`
///
/// `createExperiment` is `n(945810).mj`
#[test]
async fn finds_user_experiment() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			945810.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
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
async fn ignores_other_exports_of_create_experiment_module() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			945810.into(),
			&[ExportMapKey::Named("notMj".into())],
		)
		.await;
	assert_eq!(experiments, vec![]);
}

#[test]
async fn ignores_other_modules() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			111111.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
	assert_eq!(experiments, vec![]);
}

#[test]
async fn apex_ignores_normal_experiments() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			600975.into(),
			&[ExportMapKey::Named("C".into())],
		)
		.await;
	assert_eq!(experiments, vec![]);
}

/// module `402313` from build `6d330cc72605bec8b66e953e75d47efa2069d945`
///
/// `createExperiment` is `i(600975).C`
#[test]
async fn finds_guild_experiment() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments = p
		.get_defined_normal_experiments(
			600975.into(),
			&[ExportMapKey::Named("C".into())],
		)
		.await;
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
async fn normal_ignores_other_exports_of_create_experiment_module() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments = p
		.get_defined_normal_experiments(
			600975.into(),
			&[ExportMapKey::Named("notC".into())],
		)
		.await;
	assert_eq!(experiments, vec![]);
}

#[test]
async fn normal_ignores_other_modules() {
	let p = parse!("test_data/wp/experiments/guildExperiment.js");
	let experiments = p
		.get_defined_normal_experiments(
			111111.into(),
			&[ExportMapKey::Named("C".into())],
		)
		.await;
	assert_eq!(experiments, vec![]);
}

#[test]
async fn normal_ignores_apex_experiments() {
	let p = parse!("test_data/wp/experiments/userExperiment.js");
	let experiments = p
		.get_defined_normal_experiments(
			945810.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
	assert_eq!(experiments, vec![]);
}

/// module `932456` from build `5f9036bea3bd644a3e7f9fed68a5e30573bd4732`
///
/// `name` is a reference to a constant string
#[test]
async fn resolves_apex_experiment_name_from_identifier() {
	let p = parse!("test_data/wp/experiments/identifierNameExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			945810.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
	let [Experiment { loc, obj }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	assert_eq!(
		obj,
		&ExperimentKind::Apex(ApexExperiment {
			name: "2026-03-surface-direct-renderer".to_owned(),
			default_config: json!({ "enableSurfaceDirectRenderer": false }),
			label: None,
			variations: vec![Variation {
				key: "1".to_owned(),
				config: json!({ "enableSurfaceDirectRenderer": true }),
			}],
			kind: ExperimentScope::User,
		})
	);
	assert_eq!(loc.id, p.get_module_id().unwrap().id);
}

/// module `304476` from build `5f9036bea3bd644a3e7f9fed68a5e30573bd4732`
#[test]
async fn finds_installation_experiment() {
	let p = parse!("test_data/wp/experiments/installationExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			945810.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
	let [Experiment { obj, .. }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	assert_eq!(
		obj,
		&ExperimentKind::Apex(ApexExperiment {
			name: "2026-03-mobile-web-invite-server-profile".to_owned(),
			default_config: json!({ "enabled": false }),
			label: None,
			variations: vec![Variation {
				key: "1".to_owned(),
				config: json!({ "enabled": true }),
			}],
			kind: ExperimentScope::Installation,
		})
	);
}

/// modified from `guildExperiment.js`: `id` is a reference to a constant
/// template literal
#[test]
async fn resolves_normal_experiment_id_from_identifier() {
	let p = parse!("test_data/wp/experiments/identifierIdExperiment.js");
	let experiments = p
		.get_defined_normal_experiments(
			600975.into(),
			&[ExportMapKey::Named("C".into())],
		)
		.await;
	let [Experiment { obj, .. }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	let ExperimentKind::Normal(NormalExperiment { id, kind, .. }) = obj else {
		panic!("expected a normal experiment, got {obj:#?}");
	};
	assert_eq!(id, "2026-04_voice_user_duration");
	assert_eq!(kind, &ExperimentScope::Guild);
}

/// configs that can't be converted to json are replaced with `null`
#[test]
async fn apex_unparsable_config_is_null() {
	let p =
		parse!("test_data/wp/experiments/unparsableConfigApexExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			945810.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
	let [Experiment { obj, .. }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	assert_eq!(
		obj,
		&ExperimentKind::Apex(ApexExperiment {
			name: "2026-08-tiny-bronco".to_owned(),
			default_config: Value::Null,
			label: None,
			variations: vec![
				Variation {
					key: "0".to_owned(),
					config: json!({ "enabled": false }),
				},
				Variation {
					key: "1".to_owned(),
					config: Value::Null,
				},
			],
			kind: ExperimentScope::User,
		})
	);
}

/// configs that can't be converted to json are replaced with `null`
#[test]
async fn normal_unparsable_config_is_null() {
	let p =
		parse!("test_data/wp/experiments/unparsableConfigGuildExperiment.js");
	let experiments = p
		.get_defined_normal_experiments(
			600975.into(),
			&[ExportMapKey::Named("C".into())],
		)
		.await;
	let [Experiment { obj, .. }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	let ExperimentKind::Normal(NormalExperiment {
		id,
		default_config,
		treatments,
		..
	}) = obj
	else {
		panic!("expected a normal experiment, got {obj:#?}");
	};
	assert_eq!(id, "2026-04_voice_user_duration");
	assert_eq!(default_config, &Value::Null);
	let configs = treatments
		.iter()
		.map(|t| (t.id, &t.config))
		.collect::<Vec<_>>();
	assert_eq!(
		configs,
		vec![(1, &Value::Null), (2, &json!({ "enabled": true }))]
	);
}

/// module `860996` from build `ca2fcf2f7e54e979cdec009f6ac21320903bfde5`
///
/// `variations` is a reference to a constant object
#[test]
async fn resolves_apex_experiment_variations_from_identifier() {
	let p =
		parse!("test_data/wp/experiments/identifierVariationsExperiment.js");
	let experiments = p
		.get_defined_apex_experiments(
			945810.into(),
			&[ExportMapKey::Named("mj".into())],
		)
		.await;
	let [Experiment { obj, .. }] = experiments.as_slice() else {
		panic!("expected exactly one experiment, got {experiments:#?}");
	};
	let default =
		json!({ "showDefaultBanner": true, "showHeroPlaceholder": true });
	assert_eq!(
		obj,
		&ExperimentKind::Apex(ApexExperiment {
			name: "2026-07-quest-home-default-banner-removal".to_owned(),
			default_config: default.clone(),
			label: None,
			variations: vec![
				Variation {
					key: "0".to_owned(),
					config: default,
				},
				Variation {
					key: "1".to_owned(),
					config: json!({
						"showDefaultBanner": false,
						"showHeroPlaceholder": true,
					}),
				},
				Variation {
					key: "2".to_owned(),
					config: json!({
						"showDefaultBanner": false,
						"showHeroPlaceholder": false,
					}),
				},
			],
			kind: ExperimentScope::User,
		})
	);
}
