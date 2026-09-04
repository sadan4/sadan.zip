mod graph;

use std::{
	cell::RefCell,
	collections::HashMap,
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
use ast_parser::{get_line_and_column, get_offset_from_line_and_column};
use explorer_types::{
	DepInfo,
	FullBundle,
	IncomingModuleDeps,
	KeyModules,
	ModuleId,
	ModuleSources,
	OutgoingModuleDepsWithLocs,
	TModuleId,
};
use js_sys::{Object, Reflect, Uint32Array};
use memchr::memmem::Finder;
use miette_ctx::into_anyhow;
use oxc::{allocator::Allocator, span::Span};
use pretty_printer::{FormattedContent, format_with_alloc};
use regress::Regex;
use serde::Serialize;
use smol_str::{SmolStr, format_smolstr};
use vencord_ast_parser::patches::{
	canonicalize_intl,
	canonicalize_regex_ident,
};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use webpack_ast_parser::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
	export_map::{ExportValue, RangeExportMap, RangeExportMapValue},
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
	formatted_module_mappings: RefCell<HashMap<ModuleId, Vec<(u32, u32)>>>,
	formatted_module_line_indices: RefCell<HashMap<ModuleId, LineIndex>>,
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

struct BundleSearchResults {
	module_ids: Vec<u32>,
	raw_indices: Vec<u32>,
}

impl BundleSearchResults {
	const fn new() -> Self {
		Self {
			module_ids: Vec::new(),
			raw_indices: Vec::new(),
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleSearchResultInfo {
	line_number: u32,
	column: u32,
	preview: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleSearchLocation {
	line_number: u32,
	column: u32,
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Copy, Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
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
		// SAFETY: TODO
		let source_str =
			unsafe { mem::transmute::<&str, &'static str>(raw_source_str) };
		let mut parser = WebpackAstParser::try_new(alloc, source_str)
			.map_err(into_anyhow)
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
			let FormattedContent { code, mappings } = self.format_module(id)?;
			let boxed = Box::pin(code);
			fmt_mods.insert(id, boxed);
			self.formatted_module_mappings
				.borrow_mut()
				.insert(id, mappings);
			let ret = fmt_mods.get(&id).unwrap().as_str();
			// SAFETY: TODO
			let ret = unsafe { mem::transmute::<&str, &'a str>(ret) };
			Ok(ret)
		}
	}
	// TODO: add proper webpack header
	fn format_module(&self, id: ModuleId) -> anyhow::Result<FormattedContent> {
		let mut alloc = self.format_alloc.borrow_mut();
		alloc.reset();
		let unformatted = self
			.unformatted_modules
			.get(&id)
			.with_context(|| anyhow!("Module source not found for {id}"))?
			.as_str();
		let FormattedContent {
			mut code,
			mut mappings,
		} = format_with_alloc(unformatted, &alloc, 4)
			.context("Failed to format module")?;
		let inserted_len =
			WebpackAstParser::format_module_header(&mut code, id, false);
		if inserted_len != 0 {
			for (_, after) in &mut mappings {
				*after += inserted_len as u32;
			}
		}
		Ok(FormattedContent { code, mappings })
	}
	fn with_formatted_line_index<R>(
		&self,
		id: ModuleId,
		f: impl FnOnce(&str, &LineIndex) -> R,
	) -> anyhow::Result<R> {
		let source = self.get_formatted_module(id)?;
		let mut line_indices = self
			.formatted_module_line_indices
			.borrow_mut();
		line_indices
			.entry(id)
			.or_insert_with(|| LineIndex::new(source));
		let line_index = line_indices
			.get(&id)
			.expect("line index should exist after insertion");

		Ok(f(source, line_index))
	}
}

/// `mappings` must be sorted in ascending `before` (original position) order,
/// which is how they're built in `formatted_content_builder.rs`.
fn find_formatted_pos(mappings: &[(u32, u32)], original_pos: u32) -> u32 {
	let index = mappings.partition_point(|&(before, _)| before <= original_pos);

	let Some(&(before, after)) = index
		.checked_sub(1)
		.and_then(|i| mappings.get(i))
	else {
		return 0;
	};

	after + (original_pos - before)
}

fn normalize_source_index(source: &str, index: u32) -> u32 {
	let mut index = index.min(u32::try_from(source.len()).unwrap_or(u32::MAX));

	while !source.is_char_boundary(index as usize) {
		index -= 1;
	}

	index
}

struct LineIndex {
	line_starts: Vec<u32>,
}

impl LineIndex {
	fn new(source: &str) -> Self {
		let mut line_starts = vec![0];

		for (index, byte) in source.bytes().enumerate() {
			if byte == b'\n' {
				line_starts.push(u32::try_from(index + 1).unwrap_or(u32::MAX));
			}
		}

		Self { line_starts }
	}

	fn line_bounds(&self, source: &str, index: u32) -> (u32, u32, u32) {
		let index = normalize_source_index(source, index);
		let line_index = match self.line_starts.binary_search(&index) {
			Ok(line_index) => line_index,
			Err(line_index) => line_index.saturating_sub(1),
		};
		let line_start = self.line_starts[line_index];
		let line_end = self
			.line_starts
			.get(line_index + 1)
			.map_or_else(
				|| u32::try_from(source.len()).unwrap_or(u32::MAX),
				|line_start| line_start.saturating_sub(1),
			);
		let line_number = u32::try_from(line_index + 1).unwrap_or(u32::MAX);

		(line_start, line_end, line_number)
	}

	fn position(&self, source: &str, index: u32) -> MonacoPosition {
		let index = normalize_source_index(source, index);
		let (line_start, _, line_number) = self.line_bounds(source, index);
		let column = source[line_start as usize..index as usize]
			.chars()
			.count() + 1;
		let column = u32::try_from(column).unwrap_or(u32::MAX);

		MonacoPosition {
			line: line_number,
			column,
		}
	}

	fn preview(&self, source: &str, index: u32, long_preview: bool) -> String {
		const MAX_PREVIEW_LINES: usize = 3;
		const SHORT_PREVIEW_CHARS: u32 = 180;
		const LONG_PREVIEW_CHARS: u32 = 360;

		let index = normalize_source_index(source, index);
		let (line_start, line_end, _) = self.line_bounds(source, index);
		let (mut preview_start, mut preview_end, max_preview_chars) =
			if long_preview {
				let line_index = match self.line_starts.binary_search(&index) {
					Ok(line_index) => line_index,
					Err(line_index) => line_index.saturating_sub(1),
				};
				let start_line = line_index.saturating_sub(1);
				let end_line = (line_index + (MAX_PREVIEW_LINES - 1))
					.min(self.line_starts.len().saturating_sub(1));
				let preview_start = self.line_starts[start_line];
				let preview_end = self
					.line_starts
					.get(end_line + 1)
					.map_or_else(
						|| u32::try_from(source.len()).unwrap_or(u32::MAX),
						|line_start| line_start.saturating_sub(1),
					);

				(preview_start, preview_end, LONG_PREVIEW_CHARS)
			} else {
				(line_start, line_end, SHORT_PREVIEW_CHARS)
			};

		if preview_end.saturating_sub(preview_start) > max_preview_chars {
			preview_start = index
				.saturating_sub(max_preview_chars / 2)
				.max(preview_start);
			preview_end = preview_start
				.saturating_add(max_preview_chars)
				.min(preview_end);

			preview_start = normalize_source_index(source, preview_start);
			preview_end = normalize_source_index(source, preview_end);
		}

		source[preview_start as usize..preview_end as usize]
			.trim()
			.chars()
			.take(max_preview_chars as usize)
			.collect()
	}
}

fn formatted_search_position(
	inner: &BundleInner,
	module_id: ModuleId,
	raw_index: u32,
) -> Result<MonacoPosition> {
	let formatted_source = inner.get_formatted_module(module_id)?;
	let mappings = inner.formatted_module_mappings.borrow();
	let mappings = mappings
		.get(&module_id)
		.with_context(|| {
			format!("Module source mapping not found for {module_id}")
		})?;
	let formatted_index = find_formatted_pos(mappings, raw_index);
	let formatted_index =
		normalize_source_index(formatted_source, formatted_index);

	Ok(inner.with_formatted_line_index(
		module_id,
		|formatted_source, line_index| {
			line_index.position(formatted_source, formatted_index)
		},
	)?)
}

fn formatted_search_result_info(
	inner: &BundleInner,
	module_id: ModuleId,
	raw_index: u32,
	long_preview: bool,
) -> Result<BundleSearchResultInfo> {
	let formatted_source = inner.get_formatted_module(module_id)?;
	let mappings = inner.formatted_module_mappings.borrow();
	let mappings = mappings
		.get(&module_id)
		.with_context(|| {
			format!("Module source mapping not found for {module_id}")
		})?;
	let formatted_index = find_formatted_pos(mappings, raw_index);
	let formatted_index =
		normalize_source_index(formatted_source, formatted_index);

	Ok(inner.with_formatted_line_index(
		module_id,
		|formatted_source, line_index| {
			let position =
				line_index.position(formatted_source, formatted_index);

			BundleSearchResultInfo {
				line_number: position.line,
				column: position.column,
				preview: line_index.preview(
					formatted_source,
					formatted_index,
					long_preview,
				),
			}
		},
	)?)
}

fn push_module_search_results(
	results: &mut BundleSearchResults,
	module_id: ModuleId,
	match_indices: impl Iterator<Item = u32>,
) {
	for index in match_indices {
		results.module_ids.push(*module_id);
		results.raw_indices.push(index);
	}
}

fn search_results_to_js(results: &BundleSearchResults) -> Result<JsValue> {
	let obj = Object::new();
	let module_ids = Uint32Array::from(results.module_ids.as_slice());
	let raw_indices = Uint32Array::from(results.raw_indices.as_slice());
	Reflect::set(&obj, &JsValue::from_str("moduleIds"), module_ids.as_ref())
		.map_err(|_| anyhow!("Failed to serialize search module ids"))?;
	Reflect::set(&obj, &JsValue::from_str("rawIndices"), raw_indices.as_ref())
		.map_err(|_| anyhow!("Failed to serialize search raw indices"))?;

	Ok(obj.into())
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
	#[wasm_bindgen(skip_typescript)]
	pub fn search_modules(&self, query: &str, regex: bool) -> Result<JsValue> {
		let query = query.trim();
		let query = canonicalize_intl(query, regex, None)
			.context("Failed to canonicalize intl")?;
		let query = if regex {
			canonicalize_regex_ident(&query)
		} else {
			query
		};
		if query.is_empty() {
			return search_results_to_js(&BundleSearchResults::new());
		}

		let mut module_ids: Vec<ModuleId> = self
			.inner
			.unformatted_modules
			.keys()
			.copied()
			.collect();
		module_ids.sort_unstable();

		let mut results = BundleSearchResults::new();
		let pattern = if regex {
			Some(Regex::new(&query).context("Invalid search regex")?)
		} else {
			None
		};
		let finder = (!regex).then(|| Finder::new(query.as_bytes()));

		for module_id in module_ids {
			let source = self
				.inner
				.unformatted_modules
				.get(&module_id)
				.with_context(|| {
					anyhow!("Module source not found for {module_id}")
				})?;

			if let Some(pattern) = &pattern {
				push_module_search_results(
					&mut results,
					module_id,
					pattern
						.find_iter(source)
						.map(|regex_match| {
							u32::try_from(regex_match.start())
								.unwrap_or(u32::MAX)
						}),
				);
			} else {
				let finder = finder
					.as_ref()
					.expect("non-regex search should have a literal finder");
				push_module_search_results(
					&mut results,
					module_id,
					finder
						.find_iter(source.as_bytes())
						.map(|index| u32::try_from(index).unwrap_or(u32::MAX)),
				);
			}
		}

		search_results_to_js(&results)
	}
	pub fn get_search_result_info(
		&self,
		module_id: u32,
		raw_index: u32,
		long_preview: bool,
	) -> Result<JsValue> {
		let info = formatted_search_result_info(
			&self.inner,
			ModuleId(module_id),
			raw_index,
			long_preview,
		)?;

		Ok(serde_wasm_bindgen::to_value(&info)
			.context("Failed to serialize search result info")?)
	}
	pub fn get_search_location(
		&self,
		module_id: u32,
		raw_index: u32,
	) -> Result<JsValue> {
		let position = formatted_search_position(
			&self.inner,
			ModuleId(module_id),
			raw_index,
		)?;
		let location = BundleSearchLocation {
			line_number: position.line,
			column: position.column,
		};

		Ok(serde_wasm_bindgen::to_value(&location)
			.context("Failed to serialize search location")?)
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
		let locs = parser
			.generate_definitions(pos)
			.map_err(into_anyhow)?;
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

		let locs = parser
			.generate_references(pos)
			.map_err(into_anyhow)?;

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
			.generate_hover(pos)
			.map_err(into_anyhow)?
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

	pub fn get_module_export_map(&self, module_id: u32) -> Result<JsValue> {
		let m_id = ModuleId(module_id);
		let fmt_src = self.inner.get_formatted_module(m_id)?;
		let parser = self.inner.get_or_make_parser(m_id)?;
		let tree = build_export_tree(parser.get_export_map(), fmt_src);

		Ok(serde_wasm_bindgen::to_value(&tree)
			.context("Failed to serialize export map")?)
	}

	pub fn get_module_dependencies(&self, module_id: u32) -> Result<JsValue> {
		let m_id = ModuleId(module_id);
		let parser = self.inner.get_or_make_parser(m_id)?;
		static DEFAULT: OutgoingModuleDepsWithLocs =
			OutgoingModuleDepsWithLocs::new();
		let deps = parser
			.get_modules_that_this_module_requires()
			.unwrap_or(&DEFAULT);
		let sync_uses: Vec<ModuleId> = deps.sync.iter().map(|s| s.id).collect();
		let lazy_uses: Vec<ModuleId> = deps.lazy.iter().map(|s| s.id).collect();
		let tmp = ModuleDepsJs {
			sync_uses: &sync_uses,
			lazy_uses: &lazy_uses,
		};

		Ok(serde_wasm_bindgen::to_value(&tmp)
			.context("Failed to serialize module dependencies")?)
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportTreeNode {
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	hover: Option<String>,
	// always serialized, even when empty: the frontend (`Exports.tsx`)
	// and the `ExportTreeNode` typescript typings both read these
	// fields unconditionally (`node.children.length`, `node.ranges[0]`)
	ranges: Vec<MonacoRange>,
	children: Vec<Self>,
}

fn build_export_tree(map: &RangeExportMap, src: &str) -> Vec<ExportTreeNode> {
	let mut nodes: Vec<ExportTreeNode> = map
		.exports
		.iter()
		.map(|(k, v)| build_export_node(k.to_string(), v, src))
		.collect();

	if let Some(def) = &map.cjs_default {
		nodes.push(build_export_node("default".to_string(), def, src));
	}

	nodes.sort_by(|a, b| a.name.cmp(&b.name));
	nodes
}

fn build_export_node(
	name: String,
	value: &RangeExportMapValue,
	src: &str,
) -> ExportTreeNode {
	match value {
		ExportValue::Range(range) => ExportTreeNode {
			name,
			hover: range.1.as_ref().map(SmolStr::to_string),
			ranges: range
				.0
				.iter()
				.map(|span| MonacoRange::from_span(*span, src))
				.collect(),
			children: Vec::new(),
		},
		ExportValue::Map(map) => ExportTreeNode {
			name,
			hover: map
				.hover
				.as_ref()
				.map(SmolStr::to_string),
			ranges: Vec::new(),
			children: build_export_tree(map, src),
		},
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
    export interface BundleSearchResults {
        moduleIds: Uint32Array;
        rawIndices: Uint32Array;
    }

    export interface BundleSearchResultInfo {
        lineNumber: number;
        column: number;
        preview: string;
    }

    export interface BundleSearchLocation {
        lineNumber: number;
        column: number;
    }

    export interface MonacoRangeJs {
        start: { line: number; column: number };
        end: { line: number; column: number };
    }

    export interface ExportTreeNode {
        name: string;
        hover?: string;
        ranges: MonacoRangeJs[];
        children: ExportTreeNode[];
    }

    export interface Bundle {
        get_module_deps(module_id: number): {
            syncUses: number[];
            lazyUses: number[];
        } | undefined;
        get_module_dependencies(module_id: number): {
            syncUses: number[];
            lazyUses: number[];
        };
        search_modules(query: string, regex: boolean): BundleSearchResults;
        get_search_result_info(module_id: number, raw_index: number, long_preview: boolean): BundleSearchResultInfo;
        get_search_location(module_id: number, raw_index: number): BundleSearchLocation;
        get_module_export_map(module_id: number): ExportTreeNode[];
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
		env_var_text: _,
	}: FullBundle = fetch_struct(&FULL_BUNDLE_ENDPOINT(build_hash)).await?;
	let module_sources = if drop_sources {
		ModuleSources::default()
	} else {
		module_sources
	};
	let raw_alloc = Box::new(Allocator::new());
	let parsers = RefCell::new(HashMap::new());
	let formatted_modules = RefCell::new(HashMap::new());
	let formatted_module_mappings = RefCell::new(HashMap::new());
	let formatted_module_line_indices = RefCell::new(HashMap::new());
	let inner = BundleInner {
		metadata: metadata.into(),
		dep_info: dep_info.into(),
		module_sources,
		unformatted_modules: modules,
		raw_alloc,
		parsers,
		format_alloc: RefCell::new(Allocator::new()),
		formatted_modules,
		formatted_module_mappings,
		formatted_module_line_indices,
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

#[cfg(test)]
mod tests {
	#![allow(clippy::unreadable_literal)]

	use super::*;
	use explorer_types::{BundleMetadata, DepInfo, KeyModules};
	use std::collections::HashMap;

	fn make_test_bundle(modules: HashMap<ModuleId, String>) -> Bundle {
		let inner = BundleInner {
			metadata: BundleMetadata::default().into(),
			dep_info: DepInfo {
				key_modules: KeyModules::default(),
				module_deps: HashMap::new(),
			}
			.into(),
			module_sources: HashMap::new(),
			unformatted_modules: modules,
			raw_alloc: Box::new(Allocator::new()),
			parsers: RefCell::new(HashMap::new()),
			format_alloc: RefCell::new(Allocator::new()),
			formatted_modules: RefCell::new(HashMap::new()),
			formatted_module_mappings: RefCell::new(HashMap::new()),
			formatted_module_line_indices: RefCell::new(HashMap::new()),
			self_ptr: ptr::null(),
			_pin: PhantomPinned,
		};
		let mut inner = Box::pin(inner);
		let self_ptr = &raw const *inner;
		// SAFETY: mirrors `get_bundle`
		unsafe {
			inner
				.as_mut()
				.get_unchecked_mut()
				.self_ptr = self_ptr;
		};
		Bundle { inner }
	}

	#[test]
	fn get_module_export_map_resolves_for_cjs_default_with_dep_call() {
		let modules = HashMap::from([
			(
				ModuleId(435815),
				r#"function(e,t,n){var r=n(941094);e.exports=function(e,t){var n=e.__data__;return r(t)?n["string"==typeof t?"string":"hash"]:n.map}}"#.to_string(),
			),
			(
				ModuleId(941094),
				"function(e){e.exports=function(t){return typeof t}}".to_string(),
			),
		]);
		let bundle = make_test_bundle(modules);
		let m_id = ModuleId(435815);
		let fmt_src = bundle
			.inner
			.get_formatted_module(m_id)
			.expect("module should format");
		let parser = bundle
			.inner
			.get_or_make_parser(m_id)
			.expect("parser should be created");

		let _tree = build_export_tree(parser.get_export_map(), fmt_src);
	}

	/// The frontend (`Search.tsx`/`Exports.tsx`) and the hand-written
	/// `ExportTreeNode` typescript typings both assume `ranges` and
	/// `children` are always present arrays (never omitted), and read
	/// `node.children.length` / `node.ranges[0]` unconditionally.
	/// `#[serde(skip_serializing_if = "Vec::is_empty")]` on those fields
	/// broke that contract: a leaf node's `children` key (and a
	/// map-only node's `ranges` key) disappeared entirely from the
	/// serialized JSON, which crashed the UI with
	/// "Cannot read properties of undefined (reading 'length')".
	#[test]
	fn export_tree_node_always_serializes_ranges_and_children_arrays() {
		#[allow(clippy::literal_string_with_formatting_args)]
		let modules = HashMap::from([(
			ModuleId(1),
			r"function(e,t,n){e.exports={leaf:1,nested:{inner:2}}}".to_string(),
		)]);
		let bundle = make_test_bundle(modules);
		let m_id = ModuleId(1);
		let fmt_src = bundle
			.inner
			.get_formatted_module(m_id)
			.expect("module should format");
		let parser = bundle
			.inner
			.get_or_make_parser(m_id)
			.expect("parser should be created");
		let tree = build_export_tree(parser.get_export_map(), fmt_src);

		let json =
			serde_json::to_value(&tree).expect("export tree should serialize");
		let top_level = json
			.as_array()
			.expect("top level should be an array");

		// a leaf `Range` node must still carry a `children` array, not
		// omit the key entirely
		let leaf = top_level
			.iter()
			.find(|n| n["name"] == "leaf")
			.expect("leaf export should be present");

		assert!(
			leaf.get("children").is_some(),
			"leaf node is missing the `children` key: {leaf}"
		);
		assert_eq!(leaf["children"], serde_json::json!([]));

		// a `Map` node must still carry a `ranges` array, not omit the
		// key entirely
		let nested = top_level
			.iter()
			.find(|n| n["name"] == "nested")
			.expect("nested export should be present");

		assert!(
			nested.get("ranges").is_some(),
			"map node is missing the `ranges` key: {nested}"
		);
		assert_eq!(nested["ranges"], serde_json::json!([]));
	}

	#[test]
	fn find_formatted_pos_returns_zero_for_empty_mappings() {
		assert_eq!(find_formatted_pos(&[], 0), 0);
		assert_eq!(find_formatted_pos(&[], 42), 0);
	}

	#[test]
	fn find_formatted_pos_returns_zero_before_first_mapping() {
		let mappings = [(10, 100), (20, 250)];
		assert_eq!(find_formatted_pos(&mappings, 0), 0);
		assert_eq!(find_formatted_pos(&mappings, 9), 0);
	}

	#[test]
	fn find_formatted_pos_matches_exact_mapping_entries() {
		let mappings = [(10, 100), (20, 250), (30, 400)];
		assert_eq!(find_formatted_pos(&mappings, 10), 100);
		assert_eq!(find_formatted_pos(&mappings, 20), 250);
		assert_eq!(find_formatted_pos(&mappings, 30), 400);
	}

	#[test]
	fn find_formatted_pos_offsets_from_nearest_preceding_mapping() {
		let mappings = [(10, 100), (20, 250), (30, 400)];
		// between first and second entry: offset from (10, 100)
		assert_eq!(find_formatted_pos(&mappings, 15), 105);
		// between second and third entry: offset from (20, 250)
		assert_eq!(find_formatted_pos(&mappings, 25), 255);
		// past the last entry: offset from (30, 400)
		assert_eq!(find_formatted_pos(&mappings, 100), 470);
	}

	#[test]
	fn find_formatted_pos_matches_naive_linear_scan() {
		// reference impl mirroring the original linear-scan behavior,
		// used to fuzz-check the binary search against many mapping
		// tables and query positions.
		fn naive(mappings: &[(u32, u32)], original_pos: u32) -> u32 {
			for &(before, after) in mappings.iter().rev() {
				if original_pos >= before {
					return after + (original_pos - before);
				}
			}
			0
		}

		let mappings: Vec<(u32, u32)> = (0..500)
			.map(|i| (i * 7, i * 7 + i * 3))
			.collect();

		for original_pos in 0..(500 * 7 + 50) {
			assert_eq!(
				find_formatted_pos(&mappings, original_pos),
				naive(&mappings, original_pos),
				"mismatch at original_pos={original_pos}"
			);
		}
	}
}
