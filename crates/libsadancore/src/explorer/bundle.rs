mod graph;

use std::{
	cell::RefCell,
	collections::HashMap,
	fmt::Write as _,
	marker::PhantomPinned,
	mem,
	pin::Pin,
	ptr,
	rc::Rc,
};

use crate::{
	constants::FULL_BUNDLE_ENDPOINT,
	err::Result,
	explorer::meta::Meta,
	util::fetch_struct,
};
use anyhow::{Context, anyhow};
use arrayvec::ArrayString;
use ast_parser::{get_line_and_column, get_offset_from_line_and_column};
use explorer_types::{
	DepInfo,
	FullBundle,
	IncomingModuleDeps,
	KeyModules,
	ModuleId,
	ModuleSources,
	TModuleId,
};
use oxc::{allocator::Allocator, span::Span};
use pretty_printer::{FormattedContent, format_with_alloc};
use serde::Serialize;
use smol_str::{SmolStr, format_smolstr};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use webpack_ast_parser::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

struct RcDepInfo {
	#[expect(dead_code)]
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
	unformatted_modules: HashMap<ModuleId, String>,
	format_alloc: RefCell<Allocator>,
	#[expect(clippy::box_collection)]
	formatted_modules: RefCell<HashMap<ModuleId, Pin<Box<String>>>>,
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
		self.get_or_make_parser(id)
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
pub struct MonacoPosition {
	/// 1-based
	pub line: u32,
	/// 1-based
	pub column: u32,
}

impl Default for MonacoPosition {
	fn default() -> Self {
		Self { line: 1, column: 1 }
	}
}

#[wasm_bindgen]
impl MonacoPosition {
	/// Create a new [`MonacoPosition`]
	///
	/// line and column are 1-based
	#[wasm_bindgen(constructor)]
	pub fn new(line: u32, column: u32) -> Self {
		debug_assert_ne!(line, 0, "Line number must be greater than 0");
		debug_assert_ne!(column, 0, "Column number must be greater than 0");
		Self { line, column }
	}
	fn from_offset(src: &str, offset: u32) -> Self {
		let (line, column) = get_line_and_column(src, offset);
		Self {
			line: line + 1,
			column: column + 1,
		}
	}
	fn to_offset(self, src: &str) -> u32 {
		let Self { line, column } = self;
		get_offset_from_line_and_column(src, line - 1, column - 1)
	}
}

/// 1-based
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Default)]
pub struct MonacoRange {
	/// 1-based
	#[wasm_bindgen(readonly)]
	pub start: MonacoPosition,
	/// 1-based
	#[wasm_bindgen(readonly)]
	pub end: MonacoPosition,
}

impl MonacoRange {
	fn from_span(span: Span, src: &str) -> Self {
		Self {
			start: MonacoPosition::from_offset(src, span.start),
			end: MonacoPosition::from_offset(src, span.end),
		}
	}
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct HoverInfo {
	#[wasm_bindgen(readonly)]
	pub range: MonacoRange,
	pub(crate) content: SmolStr,
	pub(crate) i18n_key: Option<SmolStr>,
}

#[wasm_bindgen]
impl HoverInfo {
	#[wasm_bindgen(getter)]
	pub fn content(&self) -> String {
		self.content.to_string()
	}

	#[wasm_bindgen(getter)]
	pub fn i18n_key(&self) -> Option<String> {
		self.i18n_key
			.as_ref()
			.map(ToString::to_string)
	}
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
			.get_formatted_module(id)
			.context("Failed to get formatted module source")?;
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
	fn get_or_make_parser(
		&self,
		id: ModuleId,
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
	fn get_formatted_module<'a>(
		&'a self,
		id: ModuleId,
	) -> anyhow::Result<&'a str> {
		let mut fmt_mods = self.formatted_modules.borrow_mut();
		if let Some(fmt) = fmt_mods.get(&id) {
			let ret = fmt.as_str();
			// SAFETY: TODO
			let ret = unsafe { mem::transmute::<&str, &'a str>(ret) };
			Ok(ret)
		} else {
			let formatted_source = self.format_module(id)?;
			let boxed = Box::pin(formatted_source);
			fmt_mods.insert(id, boxed);
			let ret = fmt_mods.get(&id).unwrap().as_str();
			// SAFETY: TODO
			let ret = unsafe { mem::transmute::<&str, &'a str>(ret) };
			Ok(ret)
		}
	}
	// TODO: add proper webpack header
	fn format_module(&self, id: ModuleId) -> anyhow::Result<String> {
		let mut alloc = self.format_alloc.borrow_mut();
		alloc.reset();
		let unformatted = self
			.unformatted_modules
			.get(&id)
			.with_context(|| anyhow!("Module source not found for {id}"))?
			.as_str();
		let FormattedContent {
			mut code,
			mappings: _,
		} = format_with_alloc(unformatted, &alloc, 4)
			.context("Failed to format module")?;
		format_module_header(&mut code, id, false);
		Ok(code)
	}
}

fn format_module_header(src: &mut String, m_id: ModuleId, is_find: bool) {
	const BUF_LEN: usize = 128;
	if WebpackAstParser::is_webpack_module(src) {
		return;
	}
	let mut buf = ArrayString::<BUF_LEN>::new_const();
	writeln!(buf, "// Webpack Module {m_id}").unwrap();
	if is_find {
		writeln!(buf, "//OPEN FULL MODULE: {m_id}").unwrap();
	}
	writeln!(buf, "//EXTRACTED WEBPACK MODULE {m_id}").unwrap();
	writeln!(buf, "0,").unwrap();
	src.insert_str(0, &buf);
}

#[wasm_bindgen]
impl Bundle {
	pub fn get_module_text(&self, module_id: u32) -> Result<String> {
		// TODO: better errors
		let ret = self
			.inner
			.get_formatted_module(ModuleId(module_id))?
			.to_string();
		Ok(ret)
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
			.unformatted_modules
			.keys()
			.copied()
			.map(Into::into)
			.collect();
		ret.sort_unstable();
		ret.into()
	}
	pub fn has_id(&self, module_id: u32) -> bool {
		self.inner
			.unformatted_modules
			.contains_key(&ModuleId(module_id))
	}
	/// line and column are 1-based
	pub fn provide_definition(
		&mut self,
		module_id: u32,
		m_pos: MonacoPosition,
	) -> Result<Box<[ModuleLocation]>> {
		let m_id = ModuleId(module_id);
		let fmt_src = self.inner.get_formatted_module(m_id)?;
		let parser = self.inner.get_or_make_parser(m_id)?;
		let pos = m_pos.to_offset(fmt_src);
		let locs = parser.generate_definitions(pos)?;
		let ret = locs
			.into_iter()
			.map(|loc| {
				let range = match self
					.inner
					.get_formatted_module(loc.module_id)
				{
					Ok(src) => MonacoRange::from_span(loc.range, src),
					Err(_) => MonacoRange::default(),
				};
				ModuleLocation {
					id: *loc.module_id,
					range,
				}
			})
			.collect();
		Ok(ret)
	}

	pub fn provide_references(
		&mut self,
		module_id: u32,
		m_pos: MonacoPosition,
	) -> Result<Box<[ModuleLocation]>> {
		let m_id = ModuleId(module_id);
		let fmt_src = self.inner.get_formatted_module(m_id)?;
		let parser = self.inner.get_or_make_parser(m_id)?;
		let pos = m_pos.to_offset(fmt_src);

		let locs = parser.generate_references(pos)?;

		let ret = locs
			.into_iter()
			.map(|loc| {
				let range = MonacoRange::from_span(
					loc.range,
					self.inner
						.get_formatted_module(loc.module_id)
						// this should never fail since it is needed to create the parser for this module
						.unwrap(),
				);
				ModuleLocation {
					id: *loc.module_id,
					range,
				}
			})
			.collect();

		Ok(ret)
	}

	pub fn provide_hover(
		&mut self,
		module_id: u32,
		m_pos: MonacoPosition,
	) -> Result<Option<HoverInfo>> {
		let m_id = ModuleId(module_id);
		let fmt_src = self.inner.get_formatted_module(m_id)?;
		let parser = self.inner.get_or_make_parser(m_id)?;
		let pos = m_pos.to_offset(fmt_src);

		if let Some(hover) = provide_i18n_hover(fmt_src, &parser, pos) {
			return Ok(Some(hover));
		}

		let ret = parser
			.generate_hover(pos)?
			.map(|(span, content)| {
				let range = MonacoRange::from_span(span, fmt_src);
				HoverInfo {
					range,
					content,
					i18n_key: None,
				}
			});

		Ok(ret)
	}
}

fn provide_i18n_hover(
	fmt_src: &str,
	parser: &WebpackAstParser,
	pos: u32,
) -> Option<HoverInfo> {
	let (span, hashed_key) = parser.get_i18n_key_at(pos)?;

	Some(HoverInfo {
		range: MonacoRange::from_span(span, fmt_src),
		content: SmolStr::default(),
		i18n_key: Some(hashed_key),
	})
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
	let formatted_modules = RefCell::new(HashMap::new());
	let inner = BundleInner {
		metadata: metadata.into(),
		dep_info: dep_info.into(),
		module_sources,
		unformatted_modules: modules,
		raw_alloc,
		parsers,
		format_alloc: RefCell::new(Allocator::new()),
		formatted_modules,
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
	let ret = Bundle { inner };
	Ok(ret)
}
