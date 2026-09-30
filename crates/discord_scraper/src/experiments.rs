use std::{
	collections::HashMap,
	sync::{Arc, OnceLock},
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use explorer_types::{
	DepInfo,
	IncomingModuleDeps,
	ModuleId,
	Modules,
	experiments::Experiment,
};
use url::Url;
use webpack_ast_parser::{
	ThreadSafeParser,
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider, WeakCache},
};

/// An in-memory module cache over every module in a bundle
struct InMemoryBundle {
	parsers: OnceLock<HashMap<ModuleId, Arc<ThreadSafeParser>>>,
	deps: HashMap<ModuleId, Arc<IncomingModuleDeps>>,
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
		self.parsers
			.get()
			.context("Parsers not set")?
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
			.get(&id)
			.cloned()
			.unwrap_or_default())
	}
}

/// Collects every apex and normal experiment defined in `modules`, sorted by
/// location
///
/// # Errors
/// If a module fails to parse
pub async fn find_experiments(
	modules: &Modules,
	dep_info: &DepInfo,
) -> Result<Vec<Experiment>> {
	let store = Arc::new(InMemoryBundle {
		parsers: OnceLock::new(),
		deps: dep_info
			.module_deps
			.iter()
			.map(|(id, deps)| (*id, Arc::new(deps.clone())))
			.collect(),
	});
	let cache = Arc::new(WeakCache(Arc::downgrade(&store)));
	let parsers = modules
		.iter()
		.map(|(&id, src)| {
			let mut src = src.clone();
			WebpackAstParser::format_module_header(&mut src, id, false);
			let mut parser = ThreadSafeParser::new(src.into())
				.map_err(|e| anyhow!("Failed to parse module {id}: {e}"))?;
			parser.set_module_cache(cache.clone());
			parser.set_module_dep_provider(cache.clone());
			Ok((id, Arc::new(parser)))
		})
		.collect::<Result<HashMap<_, _>>>()?;
	let _ = store.parsers.set(parsers);
	let parsers = store.parsers.get().unwrap();

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
