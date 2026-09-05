use std::time::Instant;

use arrayvec::ArrayVec;
use daft::Diffable;
use explorer_types::ModuleId;
use itertools::Itertools;
use miette::Result;
use miette_ctx::ErrCtx as _;
use oxc_allocator::AllocatorPool;
use pretty_printer::format_to_str;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use smol_str::SmolStr;
use text_diff::{DiffHunk, DiffHunkKind};
use tracing::{info, warn};
use webpack_ast_parser::{WebpackAstParser, export_map::ExportMap};

use crate::{fetcher::ScrapedOutput, util::debug_module_url};

pub struct ModuleTracker<'a> {
	prev_info: PreviousModuleInfo,
	next_build: &'a ScrapedOutput,
	next_hash: &'a str,
	pool: AllocatorPool,
}

struct PreviousModuleInfo {
	module_id: ModuleId,
	/// the exports of the previous module
	export_map: ExportMap<()>,
	formatted_txt: String,
	/// the length of the *unformatted* source of the previous module
	///
	/// used to skip modules that are orders of magnitude different in size
	raw_len: usize,
	num_concatenated: u32,
}

/// the different hurestics that are used to track a module
#[derive(Default, Debug)]
#[non_exhaustive]
struct Confidence {
	/// if both share the same module id.
	/// this indicated that the filenames are the same
	same_id: bool,
	/// the change in concatenated modules between the two modules
	///
	/// a negative number means that there are less concatenated modules in the new module
	///
	/// a positive number means that there are more concatenated modules in the new module
	///
	/// zero means that the number of concatenated modules is the same
	num_concatenated: i32,
	/// the similarity of the code between the two modules
	///
	/// the diff is of the formatted code, not the unformatted code.
	/// we put the score here instead of the data because the data is tied to the lifeime of the input strings
	text_diff_score: u8,
	/// the difference in exports between the two modules
	exports: ExportDiff,
	/// the length of the module that is being compared
	new_len: usize,
	// TODO: also use the ids of imported modules
}
impl Confidence {
	/// Generates a score based on the different hurestics of the module
	/// 0 means that the two modules are exactly equal
	/// [`usize::MAX`] means that the two modules are completely different
	///
	/// in practice [`usize::MAX`] is never returned
	fn score(&self) -> usize {
		let il2 = self.new_len.ilog2() as usize;
		let mut score = 0;
		score += usize::from(self.text_diff_score) * 2 * il2;
		score += self.score_exports();
		score += self.num_concatenated.unsigned_abs() as usize * 2 * il2;
		if self.same_id {
			score = score.saturating_sub(150);
		}
		score
	}

	/// returns a number from 0 to 100 indicating how similar the two modules are
	///
	/// 0 means that the two modules are the same
	///
	/// 100 means that the two modules are completely different
	fn score_diff(diff: &[DiffHunk]) -> u8 {
		let mut good = 0;
		let mut all = 0;
		for v in diff {
			debug_assert_eq!(
				v.contents.len(),
				2,
				"diff hunk should have two contents"
			);
			if v.kind == DiffHunkKind::Matching {
				debug_assert_eq!(
					v.contents[0].len(),
					v.contents[1].len(),
					"matching diff hunk should have the same length"
				);
				good += v.contents[0].len();
				all += v.contents[0].len();
			} else {
				all += v.contents[0].len();
				all += v.contents[1].len();
			}
		}
		let ret = (good * 100) / all;
		debug_assert!(ret <= 100, "score_diff should be between 0 and 100");
		let ret = ret as u8;
		100 - ret
	}

	const fn score_exports(&self) -> usize {
		let al = self.exports.added.len();
		let rl = self.exports.removed.len();
		let ul = self.exports.unchanged.len();
		if self.exports.equal && ul > 1 {
			return 0;
		}
		let mut base = 5;
		base += al * 10;
		base += rl * 10;
		// if we just have the name of one export changing, this is even more unlikely
		if ul == 0 && al | rl == 1 {
			base *= 2;
		}
		base
	}
}

#[derive(Debug, Clone, Copy)]
pub struct TrackedModule {
	pub new_module_id: ModuleId,
	/// the result of [`Confidence::score`] for the tracked module
	///
	/// Lower is better
	pub score: usize,
}

/// TODO: Move to `webpack_ast_parser`
mod clear;

#[derive(Default, Debug)]
struct ExportDiff {
	added: Vec<SmolStr>,
	removed: Vec<SmolStr>,
	unchanged: Vec<SmolStr>,
	equal: bool,
	// /// only a shallow comparison `old.is_some() ^ new.is_some()`
	// cjs_default_changed: bool,
}

impl ExportDiff {
	fn diff(old: &ExportMap<()>, new: &ExportMap<()>) -> Self {
		if old == new {
			return Self {
				equal: true,
				..Self::default()
			};
		}
		let diff = old.exports.diff(&new.exports);
		let added = diff
			.added
			.into_keys()
			.cloned()
			.collect();
		let removed = diff
			.removed
			.into_keys()
			.cloned()
			.collect();
		let unchanged = diff
			.common
			.into_keys()
			.cloned()
			.collect();
		// let cjs_default_changed =
		// 	old.cjs_default.is_none() ^ new.cjs_default.is_none();
		Self {
			added,
			removed,
			unchanged,
			..Self::default()
		}
	}
}

fn diff_modules<'a>(old: &'a str, new: &'a str) -> u8 {
	let d = text_diff::diff([old, new]);
	Confidence::score_diff(&d)
}

pub type TrackedModules =
	ArrayVec<TrackedModule, { ModuleTracker::MAX_TRACKED_MODULES }>;

impl<'a> ModuleTracker<'a> {
	pub const MAX_TRACKED_MODULES: usize = 8;
	/// This is a hurestic. it could very easily be broken by a module being contatenated with a large module,
	///  or a module being split into multiple smaller modules.
	const MAX_LEN_RATIO: f64 = 16.0;

	/// how many times longer the longer of `new_len` and the previous module
	/// is than the shorter one
	///
	/// `1.0` means they are exactly the same length. empty modules are
	/// treated as one byte long, both to avoid dividing by zero and because
	/// [`Confidence::score`] cannot handle a zero length.
	#[expect(
		clippy::cast_precision_loss,
		reason = "oxc will refuse to parse before this ever becomes lossy"
	)]
	fn len_ratio(&self, new_len: usize) -> f64 {
		let prev_len = self.prev_info.raw_len.max(1) as f64;
		let new_len = new_len.max(1) as f64;
		prev_len.max(new_len) / prev_len.min(new_len)
	}

	/// the candidates from the new build that are worth scoring, cheapest
	/// filter first
	///
	/// the module with the same id as the previous one is always kept, as
	/// [`Confidence::same_id`] makes it a strong candidate on its own.
	fn candidates(&self) -> Vec<(&'a ModuleId, &'a String)> {
		let mut candidates = self
			.next_build
			.iter()
			.filter(|(k, v)| {
				!v.is_empty()
					&& (**k == self.prev_info.module_id
						|| self.len_ratio(v.len()) <= Self::MAX_LEN_RATIO)
			})
			.collect_vec();
		if candidates.len() >= Self::MAX_TRACKED_MODULES {
			return candidates;
		}
		// the previous module is an outlier in this build, so the window is
		// too narrow to fill the output. widen it to the closest modules by
		// length instead of returning fewer results than asked for
		candidates = self
			.next_build
			.iter()
			.filter(|(_, v)| !v.is_empty())
			.collect();
		candidates.sort_unstable_by(|(_, a), (_, b)| {
			self.len_ratio(a.len())
				.total_cmp(&self.len_ratio(b.len()))
		});
		candidates.truncate(Self::MAX_TRACKED_MODULES);
		candidates
	}

	pub fn try_new(
		prev_build: &'a ScrapedOutput,
		prev_mid: ModuleId,
		next_build: &'a ScrapedOutput,
		next_hash: &'a str,
	) -> Result<Self> {
		let pool = AllocatorPool::new(num_cpus::get());
		let alloc_guard = pool.get();
		let alloc = &*alloc_guard;
		let prev_module = prev_build
			.get(&prev_mid)
			.expect("previous module should exist");
		// get old module info
		let parser = WebpackAstParser::try_new(alloc, prev_module)
			.context("Failed to parse previous module")?;
		let formatted_txt = format_to_str(prev_module, 0)
			.context("Failed to format previous module")?;
		let prev_info = PreviousModuleInfo {
			module_id: prev_mid,
			export_map: clear::map(parser.get_export_map().clone()),
			formatted_txt,
			raw_len: prev_module.len(),
			num_concatenated: parser.num_concatenated_modules(),
		};
		drop(alloc_guard);
		let ret = Self {
			prev_info,
			next_build,
			next_hash,
			pool,
		};
		Ok(ret)
	}
	fn confidence_for(&self, k: ModuleId, v: &str) -> Result<Confidence> {
		let alloc = &*self.pool.get();
		let parser = WebpackAstParser::try_new(alloc, v)?;
		let new_export_map = clear::map(parser.get_export_map().clone());
		let num_concatenated = parser.num_concatenated_modules();
		let exports =
			ExportDiff::diff(&self.prev_info.export_map, &new_export_map);
		let new_formatted =
			format_to_str(v, 0).context("Failed to format new source")?;
		let text_diff_score =
			diff_modules(&self.prev_info.formatted_txt, &new_formatted);
		let ret = Confidence {
			same_id: k == self.prev_info.module_id,
			num_concatenated: (num_concatenated as i32)
				- (self.prev_info.num_concatenated as i32),
			text_diff_score,
			exports,
			new_len: v.len(),
		};
		Ok(ret)
	}
	pub fn track(
		self,
		bars: &crate::util::MultiProgressWrapper,
	) -> TrackedModules {
		// if let Some(new_module_contents) = self
		// 	.next_build
		// 	.get(&self.prev_info.module_id)
		// {
		// 	let c = self
		// 		.confidence_for(self.prev_info.module_id, new_module_contents)
		// 		.context("Failed to get confidence for module with same id")?;
		// 	let score = c.score();
		// 	todo!("score for module with same id: {score}");
		// }
		// let id = ModuleId(89865);
		// let m = &self.next_build[&id];
		// let c = self
		// 	.confidence_for(id, m)
		// 	.context("failed to get confidence for new module")?;
		// let score = c.score();
		// todo!("score for new module: {score}");
		let start = Instant::now();
		let candidates = self.candidates();
		info!(
			"Scoring {} of {} modules in the new build",
			candidates.len(),
			self.next_build.len()
		);
		let bar =
			crate::util::Stage::new("Tracking module", Some(candidates.len()))
				.and_attach(bars);

		let mut scores: Vec<_> = candidates
			.par_iter()
			.filter_map(|(k, v)| {
				let c = match self.confidence_for(**k, v) {
					Ok(c) => c,
					Err(e) => {
						warn!(
							"Failed to get confidence for module url=<{}>. cause: {e:?}",
							debug_module_url(**k, self.next_hash)
						);
						bar.step();
						return None;
					}
				};
				bar.step();
				Some(TrackedModule {
					new_module_id: **k,
					score: c.score(),
				})
			})
			.collect();
		let elapsed = start.elapsed();
		info!("Tracked {} modules in {:?}", scores.len(), elapsed);
		scores.sort_unstable_by_key(|module| module.score);
		scores.truncate(Self::MAX_TRACKED_MODULES);
		ArrayVec::from_iter(scores)
	}
}
