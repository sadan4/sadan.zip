use std::{collections::HashMap, hash::BuildHasher};

use anyhow::Result;
use explorer_types::{DepInfo, IncomingModuleDeps, KeyModules, ModuleId};
use miette_ctx::into_anyhow;
use oxc_allocator::Allocator;
use webpack_ast_parser::WebpackAstParser;

pub fn parse_bundle<S: BuildHasher>(
	modules: &HashMap<ModuleId, String, S>,
) -> Result<DepInfo> {
	let alloc = Allocator::new();
	let mut parsers = HashMap::with_capacity(modules.len());
	for (id, code) in modules {
		let parser =
			WebpackAstParser::try_new(&alloc, code).map_err(into_anyhow)?;
		parsers.insert(*id, parser);
	}
	let mut deps: HashMap<_, IncomingModuleDeps> =
		HashMap::with_capacity(parsers.len());
	for (id, parser) in &parsers {
		let Some(outgoing_deps) =
			parser.get_modules_that_this_module_requires()
		else {
			continue;
		};

		for sync_dep in &outgoing_deps.sync {
			deps.entry(*sync_dep)
				.or_default()
				.sync
				.push(*id);
		}

		for lazy_dep in &outgoing_deps.lazy {
			deps.entry(*lazy_dep)
				.or_default()
				.lazy
				.push(*id);
		}
	}
	Ok(DepInfo {
		key_modules: KeyModules::default(),
		module_deps: deps,
	})
}
