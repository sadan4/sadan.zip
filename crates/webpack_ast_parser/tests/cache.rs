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
use explorer_types::{IncomingModuleDeps, ModuleId};
use insta::{assert_debug_snapshot, assert_snapshot};
use itertools::Itertools;
use macros::cache_test;
use oxc::{allocator::Allocator, span::Span};
use smol_str::SmolStr;
use webpack_ast_parser::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
	export_map::{ExportValue, RangeExportMap, RangeExportMapValue},
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
	fn dbg_defs(
		&self,
		parser: &WebpackAstParser,
		line: u32,
		col: u32,
	) -> Result<Vec<DefinitionDumper<'a>>> {
		let pos =
			get_offset_from_line_and_column(parser.get_source(), line, col);
		parser
			.generate_definitions(pos)
			.map(|refs| {
				refs.into_iter()
					.map(|ref_| {
						let other_parser = self.parse(*ref_.module_id);
						DefinitionDumper {
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

fn dbg_hover<'a>(
	p: &WebpackAstParser<'a>,
	line: u32,
	col: u32,
) -> Result<Option<(SmolStr, SpanDumper<'a>)>> {
	let pos = get_offset_from_line_and_column(p.get_source(), line, col);
	let s = p.generate_hover(pos)?;
	Ok(s.map(|(s, t)| (t, SpanDumper(s, p.get_source()))))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceDumper<'a> {
	id: ModuleId,
	range: SpanDumper<'a>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DefinitionDumper<'a> {
	id: ModuleId,
	range: SpanDumper<'a>,
}

#[cache_test]
fn simple_export_in_single_file(b: &Bundle) {
	let parser = b.parse(222222);
	let locs = b.dbg_gen_refs(&parser, 6, 8).unwrap();
	assert_debug_snapshot!(locs, @r#"
	[
	    ReferenceDumper {
	        id: ModuleId(
	            111111,
	        ),
	        range: "[17:26->17:27)",
	    },
	]
	"#);
}

#[cache_test]
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
	            range: "[17:18->17:19)",
	        },
	        ReferenceDumper {
	            id: ModuleId(
	                111111,
	            ),
	            range: "[17:40->17:41)",
	        },
	        ReferenceDumper {
	            id: ModuleId(
	                999999,
	            ),
	            range: "[14:41->14:42)",
	        },
	    ],
	)
	"#);
}

/// finds all uses of a default e.exports where the exports
/// are assigned to the default export first
mod e_exports_default {
	use super::*;
	#[cache_test]
	fn test1(b: &Bundle) {
		let parser = b.parse(111113);
		let deps = parser
			.get_modules_that_require_this_module()
			.unwrap();
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
		        range: "[33:28->33:31)",
		    },
		]
		"#);
	}
	#[cache_test]
	fn test2(b: &Bundle) {
		let parser = b.parse(111113);
		let locs = b.dbg_gen_refs(&parser, 8, 8).unwrap();
		assert_debug_snapshot!(locs, @r#"
		[
		    ReferenceDumper {
		        id: ModuleId(
		            111111,
		        ),
		        range: "[34:28->34:31)",
		    },
		]
		"#);
	}
	#[cache_test]
	fn test3(b: &Bundle) {
		let parser = b.parse(111113);
		let locs = b.dbg_gen_refs(&parser, 11, 8).unwrap();
		assert_debug_snapshot!(locs, @r#"
		[
		    ReferenceDumper {
		        id: ModuleId(
		            111111,
		        ),
		        range: "[35:28->35:31)",
		    },
		]
		"#);
	}
}

#[cache_test]
fn react_class_component(b: &Bundle) {
	let parser = b.parse(555555);
	let locs = b.dbg_gen_refs(&parser, 11, 10).unwrap();
	let locs2 = b.dbg_gen_refs(&parser, 6, 8).unwrap();
	assert_eq!(locs, locs2);
	assert_debug_snapshot!(locs, @r#"
	[
	    ReferenceDumper {
	        id: ModuleId(
	            333333,
	        ),
	        range: "[14:52->14:53)",
	    },
	]
	"#);
}

mod enum_uses {
	use super::*;
	mod style_1 {
		use super::*;
		#[cache_test]
		fn uses_of_member(b: &Bundle) {
			let parser = b.parse(333333);
			let locs = b.dbg_gen_refs(&parser, 22, 27).unwrap();
			assert_debug_snapshot!(locs, @r#"
			[
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[20:25->20:29)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[45:17->45:21)",
			    },
			]
			"#);
		}
		#[cache_test]
		fn uses_of_object(b: &Bundle) {
			let parser = b.parse(333333);
			let locs = b.dbg_gen_refs(&parser, 21, 12).unwrap();
			assert_debug_snapshot!(locs, @r#"
			[
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[20:23->20:24)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[43:23->43:24)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[45:15->45:16)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[45:26->45:27)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            222222,
			        ),
			        range: "[20:23->20:24)",
			    },
			]
			"#);
		}
	}
	mod style_2 {
		use super::*;
		#[cache_test]
		fn uses_of_member(b: &Bundle) {
			let parser = b.parse(333333);
			let locs = b.dbg_gen_refs(&parser, 28, 14).unwrap();
			assert_debug_snapshot!(locs, @r#"
			[
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[20:36->20:40)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[46:28->46:32)",
			    },
			]
			"#);
		}
		#[cache_test]
		fn uses_of_object(b: &Bundle) {
			let parser = b.parse(333333);
			let locs = b.dbg_gen_refs(&parser, 26, 14).unwrap();
			assert_debug_snapshot!(locs, @r#"
			[
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[20:34->20:35)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[43:29->43:30)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[46:15->46:16)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[46:26->46:27)",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            222222,
			        ),
			        range: "[20:32->20:33)",
			    },
			]
			"#);
		}
	}
}

// FIXME: test with invalid positions (make sure no panics)
mod definitions {
	use super::*;
	mod wreq_d {
		use super::*;
		#[cache_test]
		fn simple_import(b: &Bundle) {
			let parser = b.parse(111111);
			let defs = b.dbg_defs(&parser, 23, 29).unwrap();
			assert_debug_snapshot!(defs, @r#"
			[
			    DefinitionDumper {
			        id: ModuleId(
			            222222,
			        ),
			        range: "[17:13->17:15)",
			    },
			]
			"#);
		}
		#[cache_test]
		fn simple_import_2(b: &Bundle) {
			let parser = b.parse(111111);
			let defs = b.dbg_defs(&parser, 26, 34);
			assert_debug_snapshot!(defs, @r#"
			Ok(
			    [
			        DefinitionDumper {
			            id: ModuleId(
			                333333,
			            ),
			            range: "[12:13->12:15)",
			        },
			    ],
			)
			"#);
		}
		mod enums {
			use super::*;

			mod style_1 {
				use super::*;
				#[cache_test]
				fn obj_def_from_obj_use(b: &Bundle) {
					let parser = b.parse(111111);
					let defs = b.dbg_defs(&parser, 20, 23).unwrap();
					assert_debug_snapshot!(defs, @r#"
					[
					    DefinitionDumper {
					        id: ModuleId(
					            333333,
					        ),
					        range: "[21:8->21:18)",
					    },
					]
					"#);
				}
				#[cache_test]
				fn obj_def_from_computed_access(b: &Bundle) {
					let parser = b.parse(222222);
					let defs = b.dbg_defs(&parser, 20, 23).unwrap();
					assert_debug_snapshot!(defs, @r#"
					[
					    DefinitionDumper {
					        id: ModuleId(
					            333333,
					        ),
					        range: "[21:8->21:18)",
					    },
					]
					"#);
				}
				#[cache_test]
				fn member_def_from_normal_use(b: &Bundle) {
					let parser = b.parse(111111);
					let defs = b.dbg_defs(&parser, 20, 27).unwrap();
					assert_debug_snapshot!(defs, @r#"
					[
					    DefinitionDumper {
					        id: ModuleId(
					            333333,
					        ),
					        range: "[22:24->22:30)",
					    },
					]
					"#);
				}
			}
			mod style_2 {
				use super::*;
				#[cache_test]
				fn obj_def_from_obj_use(b: &Bundle) {
					let parser = b.parse(111111);
					let defs = b.dbg_defs(&parser, 20, 34).unwrap();
					assert_debug_snapshot!(defs, @r#"
					[
					    DefinitionDumper {
					        id: ModuleId(
					            333333,
					        ),
					        range: "[26:8->26:18)",
					    },
					]
					"#);
				}
				#[cache_test]
				fn obj_def_from_computed_access(b: &Bundle) {
					let parser = b.parse(222222);
					let defs = b.dbg_defs(&parser, 20, 32).unwrap();
					assert_debug_snapshot!(defs, @r#"
					[
					    DefinitionDumper {
					        id: ModuleId(
					            333333,
					        ),
					        range: "[26:8->26:18)",
					    },
					]
					"#);
				}
				#[cache_test]
				fn member_def_from_normal_use(b: &Bundle) {
					let parser = b.parse(111111);
					let defs = b.dbg_defs(&parser, 20, 38).unwrap();
					assert_debug_snapshot!(defs, @r#"
					[
					    DefinitionDumper {
					        id: ModuleId(
					            333333,
					        ),
					        range: "[28:19->28:20)",
					    },
					]
					"#);
				}
			}
		}
	}
}
mod stores {
	use super::*;
	#[cache_test]
	fn definition_location_of_store_getter(b: &Bundle) {
		let parser = b.parse(111111);
		let defs = b.dbg_defs(&parser, 16, 27).unwrap();
		assert_debug_snapshot!(defs, @r#"
		[
		    DefinitionDumper {
		        id: ModuleId(
		            999999,
		        ),
		        range: "[8:8->8:11)",
		    },
		]
		"#);
	}
}
mod hover_text {
	use super::*;
	#[cache_test]
	fn store_in_other_module(b: &Bundle) {
		let parser = b.parse(555555);
		let hov = dbg_hover(&parser, 38, 8)
			.unwrap()
			.unwrap();
		assert_debug_snapshot!(hov, @r#"
		(
		    "MyTestingStore",
		    "[38:11->38:13)",
		)
		"#);
	}
	#[cache_test]
	fn store_in_other_module_2(b: &Bundle) {
		let parser = b.parse(111111);
		let hov = dbg_hover(&parser, 15, 23)
			.unwrap()
			.unwrap();
		let hov2 = dbg_hover(&parser, 32, 23)
			.unwrap()
			.unwrap();
		assert_debug_snapshot!(hov2, @r#"
		(
		    "MyTestingStore",
		    "[32:23->32:25)",
		)
		"#);
		assert_debug_snapshot!(hov, @r#"
		(
		    "MyTestingStore",
		    "[15:23->15:25)",
		)
		"#);
	}
}
