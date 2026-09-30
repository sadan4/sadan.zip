#![allow(clippy::unreadable_literal)]
#[allow(dead_code, reason = "shared with other integration tests")]
mod util;

use explorer_types::{
	ModuleId,
	experiments::{
		ApexExperiment,
		Experiment,
		ExperimentKind,
		NormalExperiment,
	},
};
use insta::assert_snapshot;
use itertools::Itertools;

use util::Bundle;

fn summarize_experiment(Experiment { loc, obj }: &Experiment) -> String {
	match obj {
		ExperimentKind::Apex(ApexExperiment {
			kind,
			name,
			variations,
			..
		}) => format!(
			"{}: apex {kind:?} {name} ({} variations)",
			loc.id,
			variations.len()
		),
		ExperimentKind::Normal(NormalExperiment {
			kind,
			id,
			treatments,
			..
		}) => format!(
			"{}: normal {kind:?} {id} ({} treatments)",
			loc.id,
			treatments.len()
		),
	}
}

#[macros::test]
fn collects_all_experiments_in_bundle() {
	tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.name("my-test-runtime")
		.build()
		.unwrap()
		.block_on(async {
			let b = Bundle::from_full_bundle(
				"5f9036bea3bd644a3e7f9fed68a5e30573bd4732",
			)
			.unwrap();
			let mut creators: Vec<ModuleId> = Vec::new();
			let mut experiments = Vec::new();
			for id in b.module_ids() {
				let parser = b.parse(id.0);
				let found = parser
					.parser()
					.get_all_experiments()
					.await
					.unwrap();
				if !found.is_empty() {
					creators.push(id);
				}
				experiments.extend(found);
			}
			experiments.sort_unstable_by(|a, b| {
				(a.loc.id, a.loc.span.start).cmp(&(b.loc.id, b.loc.span.start))
			});
			let count = |f: fn(&ExperimentKind) -> bool| {
				experiments
					.iter()
					.filter(|e| f(&e.obj))
					.count()
			};
			let apex = count(|k| matches!(k, ExperimentKind::Apex(_)));
			let normal = count(|k| matches!(k, ExperimentKind::Normal(_)));
			let summary = experiments
				.iter()
				.map(summarize_experiment)
				.join("\n");
			assert_snapshot!(format!(
				"creator modules: {}\n{apex} apex, {normal} normal\n\n{summary}",
				creators.iter().join(", "),
			));
		});
}
