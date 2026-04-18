use std::collections::HashMap;

use anyhow::Result;
use explorer_types::{DepInfo, IncomingModuleDeps, KeyModules, ModuleId};
use oxc_allocator::Allocator;
use webpack_ast_parser::WebpackAstParser;

// struct DepProvider(HashMap<ModuleId, Rc<IncomingModuleDeps>>);

// impl IModuleDepProvider for DepProvider {
// 	fn get_module_deps(&self, id: ModuleId) -> Result<Rc<IncomingModuleDeps>> {
// 		self.0
// 			.get(&id)
// 			.cloned()
// 			.ok_or_else(|| anyhow!("requesting deps for missing module {id:?}"))
// 	}
// }

// #[derive(Default)]
// struct CacheProvider<'a>(OnceLock<HashMap<ModuleId, Rc<WebpackAstParser<'a>>>>);

// impl<'a> IModuleCache<'a> for CacheProvider<'a> {
// 	fn get_module_filepath(&self, id: ModuleId) -> Option<SmolStr> {
// 		Some(format_smolstr!("{id}.js"))
// 	}

// 	fn get_module_parser(
// 		&self,
// 		_requestor: &WebpackAstParser<'a>,
// 		id: ModuleId,
// 		_latest: Option<bool>,
// 	) -> Result<Rc<WebpackAstParser<'a>>> {
// 		self.0
// 			.get()
// 			.unwrap()
// 			.get(&id)
// 			.cloned()
// 			.ok_or_else(|| {
// 				anyhow!("requesting parser for missing module {id:?}")
// 			})
// 	}
// }

pub fn parse_bundle(modules: &HashMap<ModuleId, String>) -> Result<DepInfo> {
	let alloc = Allocator::new();
	let mut parsers = HashMap::with_capacity(modules.len());
	for (id, code) in modules {
		let parser = WebpackAstParser::try_new(&alloc, code)?;
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
