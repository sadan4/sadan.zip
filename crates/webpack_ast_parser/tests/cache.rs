#![allow(clippy::unreadable_literal, clippy::needless_raw_string_hashes)]
use std::{
	cell::OnceCell,
	collections::HashMap,
	fmt::{self, Debug},
	fs,
	path::{Path, PathBuf},
	rc::Rc,
};

use anyhow::{Context, Result, anyhow};
use ast_parser::{get_offset_from_line_and_column, span_line_and_column};
use insta::{assert_snapshot, assert_debug_snapshot};
use itertools::Itertools;
use macros::test;
use oxc::{allocator::Allocator, span::Span};
use smol_str::SmolStr;
use webpack_ast_parser::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider, IncomingModuleDeps},
	export_map::{ExportValue, RangeExportMap, RangeExportMapValue},
	types::ModuleId,
};

struct Bundle<'a> {
	dir: PathBuf,
	_alloc: &'a Allocator,
	parsers: OnceCell<HashMap<ModuleId, Rc<WebpackAstParser<'a>>>>,
	deps: HashMap<ModuleId, Rc<IncomingModuleDeps>>,
}

impl<'a> Bundle<'a> {
	fn try_new(
		alloc: &'a Allocator,
	) -> Result<(Self, HashMap<ModuleId, WebpackAstParser<'a>>)> {
		let bundle_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("tests")
			.join(".modules");
		let mut parsers = HashMap::new();
		let mut deps: HashMap<ModuleId, IncomingModuleDeps> = HashMap::new();
		// collect parsers
		for entry in
			fs::read_dir(&bundle_dir).context("Failed to read bundle dir")?
		{
			let entry = entry?;
			if entry.file_type()?.is_dir() {
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
			let module_str = fs::read_to_string(&path)?;
			let module_str = alloc.alloc_str(module_str.as_str());
			let parser = WebpackAstParser::try_new(alloc, module_str)?;

			parsers.insert(id, parser);
		}

		for (id, parser) in &parsers {
			let Some(o_deps) = parser.get_modules_that_this_module_requires()
			else {
				continue;
			};
			for sync_dep in &o_deps.sync {
				deps.entry(*sync_dep)
					.or_default()
					.sync
					.push(*id);
			}
			for lazy_dep in &o_deps.lazy {
				deps.entry(*lazy_dep)
					.or_default()
					.lazy
					.push(*id);
			}
		}

		let ret = Self {
			dir: bundle_dir,
			_alloc: alloc,
			parsers: OnceCell::new(),
			deps: deps
				.into_iter()
				.map(|(id, deps)| (id, Rc::new(deps)))
				.collect(),
		};

		Ok((ret, parsers))
	}

	fn bind_plugins<'s: 'a>(
		&'s self,
		parsers: HashMap<ModuleId, WebpackAstParser<'a>>,
	) {
		let parsers = parsers
			.into_iter()
			.map(|(id, mut parser)| {
				parser.set_module_cache(self);
				parser.set_module_dep_provider(self);
				(id, Rc::new(parser))
			})
			.collect::<HashMap<_, _>>();
		self.parsers
			.set(parsers)
			.map_err(|_| anyhow!("Parsers already set"))
			.unwrap();
	}

	fn parse(&self, id: u32) -> Rc<WebpackAstParser<'a>> {
		self.parsers
			.get()
			.unwrap()
			.get(&id.into())
			.unwrap()
			.clone()
	}

	fn dbg_gen_refs(
		&self,
		parser: &WebpackAstParser,
		line: u32,
		col: u32,
	) -> Result<Vec<ReferenceDumper<'a>>> {
		let pos =
			get_offset_from_line_and_column(parser.get_source(), line, col);
		parser
			.generate_references(pos)
			.map(|refs| {
				refs.into_iter()
					.map(|ref_| {
						let other_parser = self.parse(*ref_.module_id);
						ReferenceDumper {
							id: ref_.module_id,
							range: SpanDumper(
								ref_.range,
								other_parser.get_source(),
							),
						}
					})
					.sorted()
					.collect()
			})
	}
}

impl<'a> IModuleCache<'a> for Bundle<'a> {
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
		_requestor: &WebpackAstParser<'a>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Rc<WebpackAstParser<'a>>> {
		self.parsers
			.get()
			.unwrap()
			.get(&id)
			.cloned()
			.context("Module ID not found in bundle")
	}
}

impl IModuleDepProvider for Bundle<'_> {
	fn get_module_deps(&self, id: ModuleId) -> Result<Rc<IncomingModuleDeps>> {
		self.deps
			.get(&id)
			.cloned()
			.context("Module ID not found in bundle")
	}
}

#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct SpanDumper<'a>(pub Span, pub &'a str);

impl Debug for SpanDumper<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let ((l1, c1), (l2, c2)) = span_line_and_column(self.1, self.0);
		write!(f, r#""[{l1}:{c1}->{l2}:{c2})""#)
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
						dbg_list.entry(&format!("[{l1}:{c1}->{l2}:{c2})"));
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
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

fn dbg_export_map(p: &WebpackAstParser) -> String {
	format!("{:#?}", ExportMapDumper(p.get_export_map(), p.get_source()))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceDumper<'a> {
	id: ModuleId,
	range: SpanDumper<'a>,
}

#[test]
fn test_cache() {
	let alloc = Allocator::new();
	let (b, parsers) = Bundle::try_new(&alloc).unwrap();
	b.bind_plugins(parsers);
	simple_export_in_single_file(&b);
	simple_export_in_many_files(&b);
	e_exports_default::test1(&b);
}

fn simple_export_in_single_file(b: &Bundle) {
	let parser = b.parse(222222);
	let locs = b.dbg_gen_refs(&parser, 6, 8).unwrap();
	assert_debug_snapshot!(locs, @r#"
	[
	    ReferenceDumper {
	        id: ModuleId(
	            111111,
	        ),
	        range: "[16:26->16:27)",
	    },
	]
	"#);
}

fn simple_export_in_many_files(b: &Bundle) {
	let parser = b.parse(222222);
	let locs = b.dbg_gen_refs(&parser, 5, 8);
	assert_debug_snapshot!(locs, @r#"
	Ok(
	    [
	        ReferenceDumper {
	            id: ModuleId(
	                111111,
	            ),
	            range: "[16:18->16:19)",
	        },
	        ReferenceDumper {
	            id: ModuleId(
	                111111,
	            ),
	            range: "[16:40->16:41)",
	        },
	        ReferenceDumper {
	            id: ModuleId(
	                999999,
	            ),
	            range: "[13:41->13:42)",
	        },
	    ],
	)
	"#);
}

/// finds all uses of a default e.exports where the exports
/// are assigned to the default export first
mod e_exports_default {
	use tracing::instrument;

	use super::*;
	#[instrument(skip_all)]
	pub fn test1(b: &Bundle) {
		let parser = b.parse(111113);
		let deps = parser.get_modules_that_require_this_module().unwrap();
		assert_debug_snapshot!(deps, @"
		IncomingModuleDeps {
		    sync: [
		        ModuleId(
		            111112,
		        ),
		    ],
		    lazy: [],
		}
		");
		let map = dbg_export_map(&parser);
		assert_snapshot!(map, @r#"
		{
		    "bar": [
		        "[8:8->8:11)",
		        "[8:11->8:14)",
		    ],
		    "baz": 2(
		        [
		            "[11:8->11:11)",
		            "[11:13->11:14)",
		        ],
		    ),
		    "foo": [
		        "[5:8->5:11)",
		        "[5:13->5:24)",
		    ],
		}
		"#);
		let locs = b.dbg_gen_refs(&parser, 5, 8).unwrap();
		assert_debug_snapshot!(locs, @r#"
		[
		    ReferenceDumper {
		        id: ModuleId(
		            111111,
		        ),
		        range: "[32:28->32:31)",
		    },
		]
		"#);
	}
}
