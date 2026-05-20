use std::{
	cell::{OnceCell, RefCell},
	collections::HashMap,
	marker::PhantomPinned,
	mem,
	pin::Pin,
	ptr::{self, NonNull},
	rc::Rc,
};

use crate::{
	constants::FULL_BUNDLE_ENDPOINT,
	err::Result,
	explorer::meta::Meta,
	util::fetch_struct,
};
use anyhow::Context;
use derive_more::Deref;
use explorer_types::{
	DepInfo,
	FullBundle,
	IncomingModuleDeps,
	KeyModules,
	ModuleId,
	ModuleSources,
	Modules,
	TModuleId,
};
use oxc::allocator::Allocator;
use serde::Serialize;
use smol_str::{SmolStr, format_smolstr};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use webpack_ast_parser::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

struct RcDepInfo {
	key_modules: KeyModules,
	module_deps: HashMap<ModuleId, Rc<IncomingModuleDeps>>,
}

impl From<DepInfo> for RcDepInfo {
	fn from(
		DepInfo {
			key_modules,
			module_deps,
		}: DepInfo,
	) -> Self {
		Self {
			key_modules,
			module_deps: module_deps
				.into_iter()
				.map(|(k, v)| (k, Rc::new(v)))
				.collect(),
		}
	}
}

#[wasm_bindgen]
pub struct Bundle {
	inner: Pin<Box<BundleInner>>,
}

struct BundleInner {
	#[expect(dead_code)]
	metadata: Meta,
	dep_info: RcDepInfo,
	#[expect(dead_code)]
	module_sources: ModuleSources,
	#[expect(clippy::box_collection)]
	modules: HashMap<ModuleId, Pin<Box<String>>>,
	raw_alloc: Box<Allocator>,
	parsers: RefCell<HashMap<ModuleId, Rc<WebpackAstParser<'static>>>>,
	self_ptr: *const Self,
	_pin: PhantomPinned,
}

impl IModuleDepProvider for BundleInner {
	fn get_module_deps(
		&self,
		id: ModuleId,
	) -> anyhow::Result<Rc<explorer_types::IncomingModuleDeps>> {
		self.dep_info
			.module_deps
			.get(&id)
			.cloned()
			.context("Module dependency info not found")
	}
}

impl IModuleCache<'static> for BundleInner {
	fn get_module_filepath(&self, id: ModuleId) -> Option<SmolStr> {
		Some(format_smolstr!("/.modules/{id}.js"))
	}

	fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'static>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> anyhow::Result<Rc<WebpackAstParser<'static>>> {
		let mut parsers = self.parsers.borrow_mut();
		if let Some(parser) = parsers.get(&id) {
			Ok(parser.clone())
		} else {
			let parser = Rc::new(self.make_parser(id)?);
			parsers.insert(id, parser.clone());
			Ok(parser)
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleDepsJs<'a> {
	sync_uses: &'a Vec<ModuleId>,
	lazy_uses: &'a Vec<ModuleId>,
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub struct MonacoRange {
	/// 1-based
	#[wasm_bindgen(readonly)]
	pub start_line: u32,
	/// 1-based
	#[wasm_bindgen(readonly)]
	pub start_column: u32,
	#[wasm_bindgen(readonly)]
	pub end_line: u32,
	#[wasm_bindgen(readonly)]
	pub end_column: u32,
}

#[wasm_bindgen]
pub struct ModuleLocation {
	#[wasm_bindgen(readonly)]
	pub id: u32,
	#[wasm_bindgen(readonly)]
	pub range: MonacoRange,
}

impl BundleInner {
	fn make_parser(&self, id: ModuleId) -> Result<WebpackAstParser<'static>> {
		let raw_alloc = &*self.raw_alloc;
		// SAFETY: TODO
		let alloc = unsafe {
			mem::transmute::<&Allocator, &'static Allocator>(raw_alloc)
		};
		let raw_source_str = self
			.modules
			.get(&id)
			.context("Module source not found")?
			.as_str();
		let source_str =
		// SAFETY: TODO
			unsafe { mem::transmute::<&str, &'static str>(raw_source_str) };
		let mut parser = WebpackAstParser::try_new(alloc, source_str)
			.context("Failed to create parser")?;
		// SAFETY: TODO
		let static_self_ref: &Self = unsafe { &*self.self_ptr };
		parser.set_module_cache(static_self_ref);
		parser.set_module_dep_provider(static_self_ref);
		Ok(parser)
	}
}

#[wasm_bindgen]
impl Bundle {
	pub fn get_module_text(&self, module_id: u32) -> Option<String> {
		self.inner
			.modules
			.get(&ModuleId(module_id))
			.map(|s| (**s).clone())
	}
	#[wasm_bindgen(skip_typescript)]
	pub fn get_module_deps(&self, module_id: u32) -> Option<JsValue> {
		let deps = self
			.inner
			.dep_info
			.module_deps
			.get(&ModuleId(module_id))?;
		let tmp = ModuleDepsJs {
			sync_uses: &deps.sync,
			lazy_uses: &deps.lazy,
		};
		let ret = serde_wasm_bindgen::to_value(&tmp).unwrap();
		Some(ret)
	}
	pub fn get_id_list(&self) -> Box<[TModuleId]> {
		let mut ret: Vec<u32> = self
			.inner
			.modules
			.keys()
			.copied()
			.map(Into::into)
			.collect();
		ret.sort_unstable();
		ret.into()
	}
	pub fn has_id(&self, module_id: u32) -> bool {
		self.inner
			.modules
			.contains_key(&ModuleId(module_id))
	}
	pub fn provide_definition(
		&mut self,
		module_id: u32,
	) -> Box<[ModuleLocation]> {
		todo!()
	}
}

#[wasm_bindgen(typescript_custom_section)]
const MODULE_DEPS_JS_TYPES: &str = r#"
    export interface Bundle {
        get_module_deps(module_id: number): {
            syncUses: number[];
            lazyUses: number[];
        } | undefined;
    }
"#;

#[wasm_bindgen]
pub async fn get_bundle(
	build_hash: &str,
	drop_sources: bool,
) -> Result<Bundle> {
	let FullBundle {
		metadata,
		dep_info,
		module_sources,
		modules,
	}: FullBundle = fetch_struct(&FULL_BUNDLE_ENDPOINT(build_hash)).await?;
	let module_sources = if drop_sources {
		ModuleSources::default()
	} else {
		module_sources
	};
	let raw_alloc = Box::new(Allocator::new());
	let parsers = RefCell::new(HashMap::new());
	let inner = BundleInner {
		metadata: metadata.into(),
		dep_info: dep_info.into(),
		module_sources,
		modules: modules
			.into_iter()
			.map(|(k, v)| (k, Box::pin(v)))
			.collect(),
		raw_alloc,
		parsers,
		self_ptr: ptr::null(),
		_pin: PhantomPinned,
	};
	let mut inner = Box::pin(inner);
	let self_ptr = &raw const *inner;
	// SAFETY: TODO
	unsafe {
		inner
			.as_mut()
			.get_unchecked_mut()
			.self_ptr = self_ptr;
	};
	todo!()
}
