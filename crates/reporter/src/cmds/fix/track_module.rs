use std::convert::Infallible;

use daft::Diffable;
use diff_match_patch_rs::{DiffMatchPatch, Ops, dmp::Diff};
use explorer_types::ModuleId;
use itertools::Itertools;
use miette::{IntoDiagnostic, Report, Result, miette};
use miette_ctx::ErrCtx as _;
use oxc_allocator::AllocatorPool;
use pretty_printer::format_to_str;
use smol_str::SmolStr;
use tracing::debug;
use webpack_ast_parser::{
	WebpackAstParser,
	export_map::{ExportMap, ExtraData, RangeExportMap},
};

use crate::fetcher::ScrapedOutput;

pub struct ModuleTracker<'a> {
	prev_build: &'a ScrapedOutput,
	prev_info: PreviousModuleInfo<'a>,
	next_build: &'a ScrapedOutput,
	pool: AllocatorPool,
}

struct PreviousModuleInfo<'a> {
	module_id: ModuleId,
	/// the exports of the previous module
	export_map: ExportMap<()>,
	len: usize,
	txt: &'a str,
	formatted_txt: String,
	num_concatenated: u32,
}

/// the different hurestics that are used to track a module
#[derive(Default, Debug)]
#[non_exhaustive]
struct Confindence {
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
	text_diff: Vec<Diff<u8>>,
	/// the difference in exports between the two modules
	exports: ExportDiff,
	/// the length of the original module
	orig_len: usize,
	/// the length of the module that is being compared
	new_len: usize,
}
impl Confindence {
	/// Generates a score based on the different hurestics of the module
	/// 0 means that the two modules are exactly equal
	/// [`u32::MAX`] means that the two modules are completely different
	fn score(&self) -> u32 {
		todo!()
	}

	fn diff_len(&self) -> isize {
		self.new_len as isize - self.orig_len as isize
	}

	/// score the diff
	fn score_diff(&self) -> u32 {
		let it = self
			.text_diff
			.iter()
			.filter(|d| d.op() == Ops::Equal)
			.map(|d| d.data().len())
			.sum::<usize>()
			* 2;
		let ret = it / (self.orig_len + self.new_len);
		debug!("ret={ret}");
		ret as u32
	}
}

#[derive(Debug)]
pub struct TrackedModule {
	new_module_id: ModuleId,
	confidence: Confindence,
}

/// TODO: Move to `webpack_ast_parser`
mod clear {
	use oxc::span::Span;
	use webpack_ast_parser::export_map::{
		ExportMap,
		ExportRange,
		ExportValue,
		ExtraData,
		RangeExportMap,
		RangeExportMapValue,
		RangeExportRange,
		StoreData,
	};

	fn store_data(
		StoreData { flux_events, .. }: StoreData<Span>,
	) -> StoreData<()> {
		StoreData {
			store: (),
			flux_events: flux_events
				.into_iter()
				.map(|(k, _)| (k, ()))
				.collect(),
		}
	}
	fn extra_data(data: ExtraData<Span>) -> ExtraData<()> {
		use ExtraData as ED;
		match data {
			ED::None => ED::None,
			ED::Store(sd) => ED::Store(store_data(sd)),
		}
	}
	fn range(ExportRange(arr, hov): RangeExportRange) -> ExportRange<()> {
		ExportRange(vec![(); arr.len()], hov)
	}
	fn value(value: ExportValue<Span>) -> ExportValue<()> {
		use ExportValue as EV;
		match value {
			EV::Range(v) => EV::Range(range(v)),
			EV::Map(v) => EV::Map(map(v)),
		}
	}
	pub fn map(
		ExportMap {
			exports,
			cjs_default,
			hover,
			extra_data: ed,
		}: ExportMap<Span>,
	) -> ExportMap<()> {
		ExportMap {
			exports: exports
				.into_iter()
				.map(|(k, v)| (k, value(v)))
				.collect(),
			cjs_default: cjs_default.map(|v| Box::new(value(*v))),
			hover,
			extra_data: extra_data(ed),
		}
	}
}

#[derive(Default, Debug)]
struct ExportDiff {
	added: Vec<SmolStr>,
	removed: Vec<SmolStr>,
	equal: bool,
	/// only a shallow comparison `old.is_some() ^ new.is_some()`
	cjs_default_changed: bool,
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
		let cjs_default_changed =
			old.cjs_default.is_none() ^ new.cjs_default.is_none();
		Self {
			added,
			removed,
			cjs_default_changed,
			..Self::default()
		}
	}
}

fn diff_modules(old: &str, new: &str) -> Result<Vec<Diff<u8>>> {
	let dmp = DiffMatchPatch::new();
	dmp.diff_main::<u8>(old, new)
		.map_err(|e| miette!("{e:?}"))
}

impl<'a> ModuleTracker<'a> {
	pub fn try_new(
		prev_build: &'a ScrapedOutput,
		prev_mid: ModuleId,
		next_build: &'a ScrapedOutput,
	) -> miette::Result<Self> {
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
			len: parser.get_source().len(),
			txt: prev_module.as_str(),
			formatted_txt,
			num_concatenated: parser.num_concatenated_modules(),
		};
		drop(alloc_guard);
		let ret = Self {
			prev_build,
			prev_info,
			next_build,
			pool,
		};
		Ok(ret)
	}
	fn confindence_for(
		&self,
		k: ModuleId,
		v: &str,
	) -> miette::Result<Confindence> {
		let alloc = &*self.pool.get();
		let parser = WebpackAstParser::try_new(alloc, v)?;
		let new_export_map = clear::map(parser.get_export_map().clone());
		let exports =
			ExportDiff::diff(&self.prev_info.export_map, &new_export_map);
		let new_formatted =
			format_to_str(v, 0).context("Failed to format new source")?;
		let text_diff =
			diff_modules(&self.prev_info.formatted_txt, &new_formatted)
				.context("Failed to diff modules")?;
		let ret = Confindence {
			same_id: k == self.prev_info.module_id,
			num_concatenated: (parser.num_concatenated_modules()
				- self.prev_info.num_concatenated) as i32,
			text_diff,
			exports,
			orig_len: self.prev_info.len,
			new_len: v.len(),
		};
		Ok(ret)
	}
	pub fn track(self) -> miette::Result<TrackedModule> {
		if let Some(new_module_contents) = self
			.next_build
			.get(&self.prev_info.module_id)
		{
			let c = self
				.confindence_for(self.prev_info.module_id, new_module_contents)
				.context("Failed to get confindence for module with same id")?;
			let score = c.score();
			todo!("score for module with same id: {score}");
		}
		todo!("else")
	}
}
