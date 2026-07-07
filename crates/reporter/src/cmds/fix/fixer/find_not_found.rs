use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use dashmap::DashMap;
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use miette_ctx::ErrCtx as _;
use oxc_allocator::{Allocator, AllocatorPool};
use rayon::iter::{
	IntoParallelIterator,
	IntoParallelRefMutIterator,
	ParallelIterator as _,
};
use tracing::{info, warn};
use webpack_ast_parser::WebpackAstParser;

use crate::{
	cmds::fix::{
		find_last_build::{BuildDiff, PreviousBundle},
		fixer::TODO,
		track_module::ModuleTracker,
	},
	diag::ReporterError,
	reporter::ReporterState,
	util::{MultiProgressWrapper, sink_sender},
	vc::Plugin,
};

#[derive(Debug)]
pub struct Fixer {
	pub diff: BuildDiff,
	pub plugins: Arc<Vec<Plugin>>,
	pub diag: ReporterError,
	pub channel: Channel,
}

impl Fixer {
	pub fn fixup_modules(&mut self) {
		fn fix_entry((k, v): (&ModuleId, &mut String)) {
			WebpackAstParser::format_module_header(v, *k, false);
		}
		info!("Fixing up module headers in working and broken builds");
		rayon::scope(|s| {
			s.spawn(|_| match &mut self.diff.broken {
				PreviousBundle::Full(full) => full
					.modules
					.par_iter_mut()
					.for_each(fix_entry),
				// a scraped bundle already is pre-processed
				PreviousBundle::Scraped(_) => {}
			});
			s.spawn(|_| {
				self.diff
					.working
					.modules
					.par_iter_mut()
					.for_each(fix_entry);
			});
		});
	}
	pub async fn fix(mut self) -> miette::Result<TODO> {
		self.fixup_modules();
		info!("Attempting to find module in new build");
		let m_id = self.find_working_module_id();
		info!("found working module in old build: {m_id:?}");
		info!("Tracking module to new build. This might take a while");
		let tracker = ModuleTracker::try_new(
			&self.diff.working.modules,
			&self.diff.working.metadata.build_hash,
			m_id,
			self.diff.broken.modules(),
			self.diff.broken.build_hash(),
		)
		.context("Failed to create module tracker")?;
		let new_module = tracker
			.track()
			.context("Failed to track module to new build")?;
		info!("new_module={new_module:?}");
		todo!()
	}

	fn find_working_module_id(&self) -> ModuleId {
		let mut tx = sink_sender(32);
		let alloc_pool = AllocatorPool::new(num_cpus::get());
		let stats = DashMap::new();
		let patch = &self.plugins[0].patches[0];
		let mut state = ReporterState {
			tx: &mut tx,
			m_bar: MultiProgressWrapper::null_bar(),
			patches: HashSet::from([patch]),
			find_map: HashMap::new(),
			alloc: alloc_pool,
			build: &self.diff.working.modules,
			stats,
			channel: self.channel,
		};
		state.collect_finds();
		debug_assert_eq!(state.find_map.len(), 1, "we only provided one patch");
		let mods = &state.find_map[patch];
		assert!(
			!mods.is_empty(),
			"the working build should not have an erroring patch"
		);
		if mods.len() > 1 {
			warn!(
				"working build has an ambiguous patch, attempting to disambiguate"
			);
			state.resolve_ambiguous_finds();
		}
		let mods = &state.find_map[patch];
		if mods.len() > 1 {
			panic!(
				"working build has an ambiguous patch that could not be disambiguated. This should not happen."
			);
		}
		mods[0]
	}
}
