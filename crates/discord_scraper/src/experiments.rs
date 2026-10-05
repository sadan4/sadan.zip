use std::{
	collections::HashMap,
	panic,
	sync::{Arc, OnceLock},
	thread,
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use explorer_types::{
	DepInfo,
	IncomingModuleDeps,
	KeyModules,
	ModuleId,
	Modules,
	experiments::Experiment,
	intl::{IntlMessage, IntlMessages},
};
use serde_json::{Map, Value};
use smol_str::SmolStr;
use tracing::warn;
use url::Url;
use webpack_ast_parser::{
	ThreadSafeParser,
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider, WeakCache},
};

type Parsers = HashMap<ModuleId, Arc<ThreadSafeParser>>;

struct InMemoryBundle {
	parsers: OnceLock<Parsers>,
	deps: OnceLock<HashMap<ModuleId, Arc<IncomingModuleDeps>>>,
}

impl InMemoryBundle {
	fn new() -> Arc<Self> {
		Arc::new(Self {
			parsers: OnceLock::new(),
			deps: OnceLock::new(),
		})
	}

	fn set_deps(&self, dep_info: &DepInfo) {
		let _ = self.deps.set(
			dep_info
				.module_deps
				.iter()
				.map(|(id, deps)| (*id, Arc::new(deps.clone())))
				.collect(),
		);
	}

	fn parsers(&self) -> Result<&Parsers> {
		self.parsers
			.get()
			.context("Parsers not set")
	}
}

#[async_trait]
impl IModuleCache for InMemoryBundle {
	async fn get_module_filepath(&self, _id: ModuleId) -> Option<Url> {
		None
	}

	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		self.parsers()?
			.get(&id)
			.cloned()
			.with_context(|| format!("Module {id} not found in bundle"))
	}
}

#[async_trait]
impl IModuleDepProvider for InMemoryBundle {
	async fn get_module_deps(
		&self,
		id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		Ok(self
			.deps
			.get()
			.context("Deps not set")?
			.get(&id)
			.cloned()
			.unwrap_or_default())
	}
}

/// Parses every module in `modules` across all available threads, wiring each
/// parser up to `store`
///
/// If `warm_deps` is set, each module's outgoing deps are computed on the
/// worker threads too, so reading them afterwards only hits the parser's cache.
fn parse_modules(
	modules: &Modules,
	store: &Arc<InMemoryBundle>,
	warm_deps: bool,
) -> Result<()> {
	let cache = Arc::new(WeakCache(Arc::downgrade(store)));
	let entries = modules.iter().collect::<Vec<_>>();
	let threads = thread::available_parallelism().map_or(1, usize::from);
	let chunk_size = entries.len().div_ceil(threads).max(1);
	let parse_chunk = |chunk: &[(&ModuleId, &String)]| {
		chunk
			.iter()
			.map(|&(&id, src)| {
				let mut src = src.clone();
				WebpackAstParser::format_module_header(&mut src, id, false);
				let src: Arc<str> = src.into();
				let mut parser =
					ThreadSafeParser::new(src.clone()).map_err(|e| {
						anyhow!(
							"{}",
							pretty_printer::render_diag(
								e,
								&src,
								&format!("{id}.js")
							)
						)
					})?;
				parser.set_module_cache(cache.clone());
				parser.set_module_dep_provider(cache.clone());
				if warm_deps {
					parser
						.parser()
						.get_modules_that_this_module_requires();
				}
				Ok((id, Arc::new(parser)))
			})
			.collect::<Result<Vec<_>>>()
	};
	let parsers = thread::scope(|s| {
		#[expect(
			clippy::needless_collect,
			reason = "all threads must be spawned before any are joined"
		)]
		let handles = entries
			.chunks(chunk_size)
			.map(|chunk| s.spawn(|| parse_chunk(chunk)))
			.collect::<Vec<_>>();
		handles
			.into_iter()
			.map(|handle| {
				handle
					.join()
					.unwrap_or_else(|payload| panic::resume_unwind(payload))
			})
			.collect::<Result<Vec<_>>>()
	})?
	.into_iter()
	.flatten()
	.collect();
	let _ = store.parsers.set(parsers);
	Ok(())
}

/// Builds the incoming dep graph from each parser's outgoing deps
fn build_dep_info(parsers: &Parsers) -> DepInfo {
	let mut deps: HashMap<_, IncomingModuleDeps> =
		HashMap::with_capacity(parsers.len());
	for (id, parser) in parsers {
		let Some(outgoing_deps) = parser
			.parser()
			.get_modules_that_this_module_requires()
		else {
			continue;
		};

		for sync_dep in &outgoing_deps.sync {
			deps.entry(sync_dep.id)
				.or_default()
				.sync
				.push(*id);
		}

		for lazy_dep in &outgoing_deps.lazy {
			deps.entry(lazy_dep.id)
				.or_default()
				.lazy
				.push(*id);
		}
	}
	DepInfo {
		key_modules: KeyModules::default(),
		module_deps: deps,
	}
}

/// Collects every experiment defined in `parsers`, sorted by location
async fn collect_experiments(parsers: &Parsers) -> Result<Vec<Experiment>> {
	let mut experiments = Vec::new();
	for parser in parsers.values() {
		experiments.extend(
			parser
				.parser()
				.get_all_experiments()
				.await
				.map_err(|e| anyhow!("{e}"))?,
		);
	}
	experiments.sort_unstable_by(|a, b| {
		(a.loc.id, a.loc.span.start).cmp(&(b.loc.id, b.loc.span.start))
	});
	Ok(experiments)
}

/// Every module in a bundle, parsed once, along with the bundle's dep graph
pub struct ParsedBundle {
	store: Arc<InMemoryBundle>,
	dep_info: DepInfo,
}

impl ParsedBundle {
	/// Parses every module in `modules` and builds the bundle's dep graph
	///
	/// # Errors
	/// If a module fails to parse
	pub fn new(modules: &Modules) -> Result<Self> {
		let store = InMemoryBundle::new();
		parse_modules(modules, &store, true)?;
		let dep_info = build_dep_info(store.parsers()?);
		store.set_deps(&dep_info);
		Ok(Self { store, dep_info })
	}

	#[must_use]
	pub const fn dep_info(&self) -> &DepInfo {
		&self.dep_info
	}

	#[must_use]
	pub fn into_dep_info(self) -> DepInfo {
		self.dep_info
	}

	/// Collects every apex and normal experiment defined in the bundle, sorted
	/// by location
	///
	/// # Errors
	/// If collecting a module's experiments fails
	pub async fn find_experiments(&self) -> Result<Vec<Experiment>> {
		collect_experiments(self.store.parsers()?).await
	}

	/// Collects every english intl module in the bundle, sorted by id
	///
	/// # Errors
	/// If no module defines `createLoader`, or collecting from it fails
	pub async fn find_intl_modules(&self) -> Result<Vec<ModuleId>> {
		collect_intl_modules(self.store.parsers()?).await
	}

	/// Collects every english intl module in the bundle, sorted by id, along
	/// with every message they define
	///
	/// # Errors
	/// If no module defines `createLoader`, or collecting from it fails
	pub async fn find_intl(&self) -> Result<IntlMessages> {
		collect_intl(self.store.parsers()?).await
	}

	pub fn get_parser(&self, id: ModuleId) -> Option<Arc<ThreadSafeParser>> {
		self.store
			.parsers()
			.ok()?
			.get(&id)
			.cloned()
	}
}

/// Collects every english intl module in `parsers`, sorted by id
async fn collect_intl_modules(parsers: &Parsers) -> Result<Vec<ModuleId>> {
	for parser in parsers.values() {
		let Some(mut ids) = parser
			.parser()
			.collect_intl_modules()
			.await
			.map_err(|e| anyhow!("{e}"))?
		else {
			continue;
		};
		ids.sort_unstable();
		ids.dedup();
		return Ok(ids);
	}
	Err(anyhow!("No module defines createLoader"))
}

/// Collects every english intl module in `parsers`, sorted by id, along with
/// every message they define
///
/// Modules whose JSON can't be read are skipped, but stay in
/// [`IntlMessages::modules`]
async fn collect_intl(parsers: &Parsers) -> Result<IntlMessages> {
	let modules = collect_intl_modules(parsers).await?;
	let mut messages = HashMap::new();
	for &id in &modules {
		let Some(parser) = parsers.get(&id) else {
			warn!(%id, "Intl module not found in bundle");
			continue;
		};
		let Some(json) = parser.parser().as_json_module() else {
			warn!(%id, "Intl module is not a JSON module");
			continue;
		};
		let json = match serde_json::from_str::<Map<String, Value>>(json) {
			Ok(json) => json,
			Err(e) => {
				warn!(%id, "Failed to parse intl module JSON: {e}");
				continue;
			}
		};
		for (key, value) in json {
			let key = SmolStr::from(key);
			let msg = IntlMessage { module: id, value };
			if let Some(prev) = messages.insert(key.clone(), msg) {
				warn!(
					%key,
					"Intl key defined in both {} and {id}, keeping {id}",
					prev.module
				);
			}
		}
	}
	Ok(IntlMessages { modules, messages })
}

/// Collects every apex and normal experiment defined in `modules`, sorted by
/// location
///
/// Use [`ParsedBundle`] instead when the dep graph has not been built yet, so
/// the modules are only parsed once.
///
/// # Errors
/// If a module fails to parse
pub async fn find_experiments(
	modules: &Modules,
	dep_info: &DepInfo,
) -> Result<Vec<Experiment>> {
	let store = InMemoryBundle::new();
	store.set_deps(dep_info);
	parse_modules(modules, &store, false)?;
	collect_experiments(store.parsers()?).await
}

/// Collects every english intl module in `modules`, sorted by id, along with
/// every message they define
///
/// Use [`ParsedBundle`] instead when the dep graph has not been built yet, so
/// the modules are only parsed once.
///
/// # Errors
/// If a module fails to parse, no module defines `createLoader`, or collecting
/// from it fails
pub async fn find_intl(
	modules: &Modules,
	dep_info: &DepInfo,
) -> Result<IntlMessages> {
	let store = InMemoryBundle::new();
	store.set_deps(dep_info);
	parse_modules(modules, &store, false)?;
	collect_intl(store.parsers()?).await
}
