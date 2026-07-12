use std::{
	cmp::Reverse,
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Instant,
};

use dashmap::DashMap;
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use itertools::Itertools;
use miette_ctx::ErrCtx as _;
use oxc_allocator::AllocatorPool;
use rayon::iter::{
	IntoParallelRefIterator as _,
	IntoParallelRefMutIterator as _,
	ParallelIterator as _,
};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use webpack_ast_parser::{WebpackAstParser, find::ScoredFindSequence};

use crate::{
	cmds::fix::{
		find_last_build::{BuildDiff, PreviousBundle},
		track_module::ModuleTracker,
	},
	diag::ReporterError,
	reporter::{Msg, ReporterState},
	util::{MultiProgressWrapper, debug_module_url, sink_sender},
	vc::Plugin,
};

#[derive(Debug)]
pub struct Fixer {
	diff: BuildDiff,
	plugins: Arc<Vec<Plugin>>,
	// diag: ReporterError,
	channel: Channel,
	tx: mpsc::Sender<Msg>,
	bars: MultiProgressWrapper,
}

impl Fixer {
	pub fn new(
		diff: BuildDiff,
		plugins: Arc<Vec<Plugin>>,
		_diag: ReporterError,
		channel: Channel,
		bars: MultiProgressWrapper,
	) -> Self {
		Self {
			diff,
			plugins,
			channel,
			tx: sink_sender(32),
			bars,
		}
	}
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

	fn new_reporter(&self) -> ReporterState<'_> {
		ReporterState {
			tx: &self.tx,
			m_bar: MultiProgressWrapper::null_bar(),
			patches: HashSet::new(),
			find_map: HashMap::new(),
			alloc: AllocatorPool::new(
				num_cpus::get().max(ModuleTracker::MAX_TRACKED_MODULES),
			),
			build: self.diff.broken.modules(),
			stats: DashMap::new(),
			channel: self.channel,
		}
	}

	pub fn fix(mut self) -> miette::Result<i8> {
		self.fixup_modules();
		info!("Attempting to find module in new build");
		let m_id = self.find_working_module_id();
		info!("found working module in old build: {m_id:?}");
		info!("Tracking module to new build. This might take a while");
		let tracker = ModuleTracker::try_new(
			&self.diff.working.modules,
			m_id,
			self.diff.broken.modules(),
			self.diff.broken.build_hash(),
		)
		.context("Failed to create module tracker")?;
		let reporter = self.new_reporter();
		let new_modules = tracker.track(&self.bars);
		let mut tested_modules: Vec<_> = new_modules
			.par_iter()
			.map(|tm| {
				let status = reporter.test_patch_against_module(
					&self.plugins[0].patches[0],
					tm.new_module_id,
					None,
				);
				(*tm, status)
			})
			.collect();
		let good_modules = tested_modules
			.extract_if(.., |(_, s)| s.is_ok())
			.collect_vec();
		let bad_modules = tested_modules;
		_ = bad_modules;
		match good_modules.len() {
			0 => {
				error!("No patches worked in the new build.");
				info!(
					"The modules tested were (a lower score is better): {new_modules:#?}"
				);
				Ok(-1)
			}
			1 => {
				let mid = good_modules[0].0.new_module_id;
				info!(
					"found new module {} it is the only one that worked",
					debug_module_url(mid, self.diff.broken.build_hash())
				);
				info!("generating new find for the module");
				let src = &self.diff.broken.modules()[&mid];
				let start = Instant::now();
				let mut good_finds = self.generate_good_finds(mid);
				good_finds.sort_by_key(|f| Reverse(f.score));
				let good_find_strs = good_finds
					.iter()
					.map(|f| f.get_find(src))
					.collect_vec();
				info!(
					"generated {} finds in {:.2?}. finds: {:#?}",
					good_find_strs.len(),
					start.elapsed(),
					good_find_strs
				);
				Ok(0)
			}
			n => {
				error!("handle {n} modules worked 100%");
				Ok(-1)
			}
		}
	}

	fn generate_good_finds(&self, mid: ModuleId) -> Vec<ScoredFindSequence> {
		crate::util::generate_unique_finds(
			mid,
			self.diff.broken.modules(),
			&crate::util::MultiProgressWrapper::null_bar(),
		)
		.unwrap()
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
