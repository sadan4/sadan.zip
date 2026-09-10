use std::{
	borrow::Cow,
	collections::HashMap,
	fmt::{self, Debug},
	fs,
	path::{Path, PathBuf},
	sync::{Arc, OnceLock},
};

use ast_parser::{get_offset_from_line_and_column, span_line_and_column};
use explorer_types::{IncomingModuleDeps, ModuleId};
use itertools::Itertools;
use miette::{Result, miette};
use miette_ctx::{ErrCtx as _, into_anyhow};
use oxc::span::Span;
use parser_diag::PResult;
use smol_str::SmolStr;
use webpack_ast_parser::{
	ThreadSafeParser,
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
	export_map::{ExportValue, RangeExportMap, RangeExportMapValue},
};

pub struct Bundle {
	dir: PathBuf,
	parsers: OnceLock<HashMap<ModuleId, Arc<ThreadSafeParser>>>,
	deps: HashMap<ModuleId, Arc<IncomingModuleDeps>>,
}

impl Bundle {
	pub fn try_new<'b>(
		sub_dir: impl Into<Option<Cow<'b, str>>>,
	) -> Result<&'static Self> {
		let mut bundle_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("tests")
			.join(".modules");
		if let Some(sub_dir) = sub_dir.into() {
			bundle_dir.push(&*sub_dir);
		}
		let mut parsers = HashMap::new();
		let mut deps: HashMap<ModuleId, IncomingModuleDeps> = HashMap::new();
		// collect parsers
		for entry in
			fs::read_dir(&bundle_dir).context("Failed to read bundle dir")?
		{
			let entry = entry.context("Failed to read entry")?;
			if entry
				.file_type()
				.context("Failed to get file type")?
				.is_dir()
			{
				continue;
			}
			let path = entry.path();
			if path
				.extension()
				.is_none_or(|ext| ext != "js")
			{
				continue;
			}
			let id: ModuleId = path
				.file_stem()
				.unwrap()
				.to_str()
				.context("Module path is not utf-8")?
				.parse::<u32>()
				.context("Module filename is not a valid ModuleId")?
				.into();
			let module_str = fs::read_to_string(&path).with_context(|| {
				format!("Failed to read {}", path.display())
			})?;
			let parser = ThreadSafeParser::new(module_str.into())?;

			parsers.insert(id, parser);
		}

		for (id, parser) in &parsers {
			let Some(o_deps) = parser
				.parser()
				.get_modules_that_this_module_requires()
			else {
				continue;
			};
			for sync_dep in &o_deps.sync {
				deps.entry(sync_dep.id)
					.or_default()
					.sync
					.push(*id);
			}
			for lazy_dep in &o_deps.lazy {
				deps.entry(lazy_dep.id)
					.or_default()
					.lazy
					.push(*id);
			}
		}

		// TODO: fix leak?
		let ret: &'static Self = Box::leak(Box::new(Self {
			dir: bundle_dir,
			parsers: OnceLock::new(),
			deps: deps
				.into_iter()
				.map(|(id, deps)| (id, Arc::new(deps)))
				.collect(),
		}));

		let parsers = parsers
			.into_iter()
			.map(|(id, mut parser)| {
				parser.set_module_cache(Arc::new(ret));
				parser.set_module_dep_provider(Arc::new(ret));
				(id, Arc::new(parser))
			})
			.collect::<HashMap<_, _>>();
		ret.parsers
			.set(parsers)
			.map_err(|_| miette!("Parsers already set"))
			.unwrap();

		Ok(ret)
	}

	pub fn parse(&self, id: u32) -> Arc<ThreadSafeParser> {
		self.parsers
			.get()
			.unwrap()
			.get(&id.into())
			.unwrap()
			.clone()
	}

	/// The source of `id`
	pub fn module_source(&self, id: ModuleId) -> &str {
		self.parsers
			.get()
			.unwrap()
			.get(&id)
			.unwrap()
			.parser()
			.get_source()
	}

	/// line and col are 0-based
	pub fn dbg_gen_refs(
		&self,
		parser: &ThreadSafeParser,
		line: u32,
		col: u32,
	) -> PResult<Vec<ReferenceDumper<'_>>> {
		let parser = parser.parser();
		let pos =
			get_offset_from_line_and_column(parser.get_source(), line, col);
		parser
			.generate_references(pos)
			.map(|refs| {
				refs.into_iter()
					.map(|ref_| ReferenceDumper {
						id: ref_.module_id,
						range: SpanDumper(
							ref_.range,
							self.module_source(ref_.module_id),
						),
					})
					.sorted()
					.collect()
			})
	}
	pub fn dbg_defs(
		&self,
		parser: &ThreadSafeParser,
		line: u32,
		col: u32,
	) -> PResult<Vec<DefinitionDumper<'_>>> {
		let parser = parser.parser();
		let pos =
			get_offset_from_line_and_column(parser.get_source(), line, col);
		parser
			.generate_definitions(pos)
			.map(|refs| {
				refs.into_iter()
					.map(|ref_| DefinitionDumper {
						id: ref_.module_id,
						range: SpanDumper(
							ref_.range,
							self.module_source(ref_.module_id),
						),
					})
					.sorted()
					.collect()
			})
	}
}

impl IModuleCache for Bundle {
	fn get_module_filepath(&self, id: ModuleId) -> Option<SmolStr> {
		Some(
			self.dir
				.join(format!("{id}.js"))
				.to_string_lossy()
				.into(),
		)
	}

	fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> anyhow::Result<Arc<ThreadSafeParser>> {
		self.parsers
			.get()
			.unwrap()
			.get(&id)
			.cloned()
			.context("Module ID not found in bundle")
			.map_err(into_anyhow)
	}
}

impl IModuleDepProvider for Bundle {
	fn get_module_deps(
		&self,
		id: ModuleId,
	) -> anyhow::Result<Arc<IncomingModuleDeps>> {
		self.deps
			.get(&id)
			.cloned()
			.context("Module ID not found in bundle")
			.map_err(into_anyhow)
	}
}

#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SpanDumper<'a>(pub Span, pub &'a str);

impl Debug for SpanDumper<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ((l1, c1), (l2, c2)) = span_line_and_column(self.1, self.0);
		let text = self.1[self.0].escape_debug();
		write!(f, r#""[{l1}:{c1}->{l2}:{c2}) {text}""#)
	}
}

struct ExportMapDumper<'a>(pub &'a RangeExportMap, pub &'a str);

impl ExportMapDumper<'_> {
	fn handle_value(
		&self,
		f: &mut fmt::Formatter<'_>,
		v: &RangeExportMapValue,
	) -> Result<(), fmt::Error> {
		match v {
			ExportValue::Range(range) => {
				let do_dbg_list = fmt::from_fn(|f| {
					let mut dbg_list = f.debug_list();
					for &span in range.iter() {
						let ((l1, c1), (l2, c2)) =
							span_line_and_column(self.1, span);
						let text = self.1[span].escape_debug();
						dbg_list
							.entry(&format!("[{l1}:{c1}->{l2}:{c2}) {text}"));
					}
					dbg_list.finish()
				});
				if let Some(hover) = &range.1 {
					f.debug_tuple(hover.as_str())
						.field(&do_dbg_list)
						.finish()
				} else {
					do_dbg_list.fmt(f)
				}
			}
			ExportValue::Map(m) => {
				let dumper = ExportMapDumper(m, self.1);
				f.debug_tuple(
					m.hover
						.as_ref()
						.map_or("ExportMap", SmolStr::as_str),
				)
				.field(&dumper)
				.finish()
			}
		}
	}
}

impl Debug for ExportMapDumper<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut dbg_map = f.debug_map();
		for (k, v) in self
			.0
			.exports
			.iter()
			.sorted_by(|a, b| a.0.cmp(b.0))
		{
			dbg_map.entry(&k, &fmt::from_fn(|f| self.handle_value(f, v)));
		}
		if let Some(v) = &self.0.cjs_default {
			let v = v.as_ref();
			dbg_map.entry(
				&"SYM_CJS_DEFAULT",
				&fmt::from_fn(|f| self.handle_value(f, v)),
			);
		}

		dbg_map.finish()
	}
}

pub fn dbg_export_map(p: &ThreadSafeParser) -> String {
	let p = p.parser();
	format!("{:#?}", ExportMapDumper(p.get_export_map(), p.get_source()))
}

pub fn dbg_hover(
	p: &ThreadSafeParser,
	line: u32,
	col: u32,
) -> Result<Option<(SmolStr, SpanDumper<'_>)>> {
	let p = p.parser();
	let pos = get_offset_from_line_and_column(p.get_source(), line, col);
	let s = p.generate_hover(pos)?;
	Ok(s.map(|(s, t)| (t, SpanDumper(s, p.get_source()))))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceDumper<'a> {
	id: ModuleId,
	range: SpanDumper<'a>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionDumper<'a> {
	id: ModuleId,
	range: SpanDumper<'a>,
}
