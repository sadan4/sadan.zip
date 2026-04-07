#![allow(clippy::unreadable_literal, clippy::too_many_lines)]
use super::*;
use ast_parser::span_line_and_column;
use insta::assert_debug_snapshot;
use itertools::Itertools;
use oxc::span::{Atom, Span};
use std::fmt::{self, Debug};

macro_rules! parse {
	($alloc:expr, $source:literal) => {{
		let source = include_str!($source);
		WebpackAstParser::try_new(&$alloc, source).unwrap()
	}};
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
				f.debug_tuple("ExportMap")
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

impl<'ast> WebpackAstParser<'ast> {
	fn t_sym_info<'a>(&'a self, sym_id: SymbolId) -> (Atom<'a>, Span)
	where
		'ast: 'a,
	{
		let name = self
			.sema
			.scoping()
			.symbol_ident(sym_id)
			.as_atom();
		let span = self
			.sema
			.scoping()
			.symbol_declaration(sym_id);
		let node = self.n(span);
		(name, node.span())
	}
	fn dbg_export_map(&self) -> ExportMapDumper<'_> {
		ExportMapDumper(self.get_export_map(), self.source)
	}
}

#[test]
fn constructs() {
	let alloc = Allocator::new();
	let source = include_str!("test_data/wp/module.js");
	_ = WebpackAstParser::try_new(&alloc, source).unwrap();
}

#[test]
fn finds_wreq() {
	let alloc = Allocator::new();
	let p = parse!(alloc, "test_data/wp/module.js");
	let wreq = p.wreq().unwrap();
	let info = p.t_sym_info(wreq);
	assert_debug_snapshot!(info, @r#"
		(
		    "n",
		    Span {
		        start: 56,
		        end: 57,
		    },
		)
		"#);
}

#[test]
fn doesnt_find_wreq_in_module_that_doesnt_use_it() {
	let alloc = Allocator::new();
	let p = parse!(alloc, "test_data/wp/bad/noWreq.js");
	assert_eq!(p.wreq(), None);
}

#[test]
fn finds_imported_var() {
	let alloc = Allocator::new();
	let p = parse!(alloc, "test_data/wp/module.js");
	let info = p
		.get_imported_var(200651.into())
		.unwrap();
	let info = p.t_sym_info(info);
	assert_debug_snapshot!(info, @r#"
		(
		    "r",
		    Span {
		        start: 181,
		        end: 194,
		    },
		)
		"#);
}

#[test]
fn doesnt_find_side_effect_import() {
	let alloc = Allocator::new();
	let p = parse!(alloc, "test_data/wp/module.js");
	let info = p.get_imported_var(411104.into());
	assert_eq!(info, None);
}

mod module_id {
	use super::*;

	#[test]
	fn parses_module_id() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/module.js");
		let id = p.get_module_id();

		assert_eq!(id, Some(ModuleId(317269)));
	}

	#[test]
	fn fails_to_parse_malformed_module_id() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/bad/badModule1.js");
		let id = p.get_module_id();
		assert_eq!(id, None);
	}

	#[test]
	fn fails_to_parse_missing_module_id() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/bad/badModule2.js");
		let id = p.get_module_id();
		assert_eq!(id, None);
	}
}
mod export_parsing {
	use super::*;
	mod wreq_d {
		use super::*;
		#[test]
		fn simple_modules() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/module.js");
			let export_map = p.dbg_export_map();
			assert_debug_snapshot!(export_map, @r#"
				{
				    "TB": [
				        "[4:8->4:10)",
				        "[162:13->162:14)",
				    ],
				    "VY": [
				        "[5:8->5:10)",
				        "[183:13->183:14)",
				    ],
				    "ZP": [
				        "[6:8->6:10)",
				        "[87:13->87:14)",
				    ],
				}
				"#);
		}
		#[test]
		fn string_literal_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/wreq.d/simpleString.js");
			let export_map = p.dbg_export_map();
			assert_debug_snapshot!(export_map, @r#"
				{
				    "STRING_EXPORT": "47835198259242069"(
				        [
				            "[5:8->5:21)",
				            "[7:12->7:31)",
				        ],
				    ),
				}
				"#);
		}

		#[test]
		fn object_literal_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/wreq.d/objectExport.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
				{
				    "EO": [
				        "[5:8->5:10)",
				        "[124:13->124:14)",
				    ],
				    "ZP": ExportMap(
				        {
				            "getFormattedName": [
				                "[164:8->164:24)",
				                "[81:13->81:14)",
				            ],
				            "getGlobalName": [
				                "[165:8->165:21)",
				                "[72:13->72:14)",
				            ],
				            "getName": [
				                "[156:8->156:15)",
				                "[53:13->53:14)",
				            ],
				            "getUserTag": [
				                "[159:8->159:18)",
				                "[142:13->142:14)",
				            ],
				            "humanizeStatus": [
				                "[166:8->166:22)",
				                "[90:13->90:14)",
				            ],
				            "isNameConcealed": [
				                "[158:8->158:23)",
				                "[158:25->158:30)",
				            ],
				            "useDirectMessageRecipient": [
				                "[167:8->167:33)",
				                "[147:13->147:14)",
				            ],
				            "useName": [
				                "[157:8->157:15)",
				                "[62:13->62:14)",
				            ],
				            "useUserTag": [
				                "[160:8->160:18)",
				                "[160:20->160:35)",
				            ],
				        },
				    ),
				}
				"#);
		}

		#[test]
		fn object_with_computed_prop() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/wreq.d/computedPropInObj.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
				{
				    "Z": ExportMap(
				        {
				            "n(231338).Et.GET_PLATFORM_BEHAVIORS": ExportMap(
				                {
				                    "handler": [
				                        "[8:12->8:19)",
				                        "[8:21->8:27)",
				                    ],
				                },
				            ),
				        },
				    ),
				}
				"#);
		}

		#[test]
		fn class_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/wreq.d/classExport.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
				{
				    "U": ExportMap(
				        {
				            "_dispatch": [
				                "[112:8->112:17)",
				            ],
				            "_dispatchWithDevtools": [
				                "[88:8->88:29)",
				            ],
				            "_dispatchWithLogging": [
				                "[91:8->91:28)",
				            ],
				            "addDependencies": [
				                "[150:8->150:23)",
				            ],
				            "addInterceptor": [
				                "[127:8->127:22)",
				            ],
				            "createToken": [
				                "[147:8->147:19)",
				            ],
				            "dispatch": [
				                "[38:8->38:16)",
				            ],
				            "dispatchForStoreTest": [
				                "[55:8->55:28)",
				            ],
				            "flushWaitQueue": [
				                "[60:8->60:22)",
				            ],
				            "isDispatching": [
				                "[35:8->35:21)",
				            ],
				            "register": [
				                "[144:8->144:16)",
				            ],
				            "subscribe": [
				                "[134:8->134:17)",
				            ],
				            "unsubscribe": [
				                "[139:8->139:19)",
				            ],
				            "wait": [
				                "[130:8->130:12)",
				            ],
				            "SYM_CJS_DEFAULT": [
				                "[5:8->5:9)",
				                "[34:10->34:11)",
				                "[153:8->153:19)",
				            ],
				        },
				    ),
				}
				"#);
		}

		#[test]
		fn enum_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/wreq.d/enums.js");
			let map = p.get_export_map();
			// only pick the keys we have tests for in js
			// TODO: Broaden tests in this module
			let mut map2 = map.clone();
			map2.exports.retain(|k, _| {
				matches!(k.as_str(), "$7" | "$X" | "$n" | "C" | "Cj" | "Si")
			});
			let map2_dumper = ExportMapDumper(&map2, p.source);
			assert_debug_snapshot!(map2_dumper, @r#"
				{
				    "$7": 28(
				        [
				            "[5:8->5:10)",
				            "[385:12->385:14)",
				        ],
				    ),
				    "$X": "1397626558063050855"(
				        [
				            "[7:8->7:10)",
				            "[421:13->421:34)",
				        ],
				    ),
				    "$n": 190(
				        [
				            "[9:8->9:10)",
				            "[739:13->739:16)",
				        ],
				    ),
				    "C": ExportMap(
				        {
				            "PREMIUM_DISCOUNT": 1(
				                [
				                    "[118:12->118:28)",
				                    "[118:31->118:32)",
				                ],
				            ),
				            "PREMIUM_TRIAL": 0(
				                [
				                    "[117:19->117:32)",
				                    "[117:35->117:36)",
				                ],
				            ),
				            "SYM_CJS_DEFAULT": [
				                "[13:8->13:9)",
				                "[116:8->116:9)",
				            ],
				        },
				    ),
				    "Cj": ExportMap(
				        {
				            "BOX": 2(
				                [
				                    "[701:12->701:15)",
				                    "[701:18->701:19)",
				                ],
				            ),
				            "CAKE": 5(
				                [
				                    "[704:12->704:16)",
				                    "[704:19->704:20)",
				                ],
				            ),
				            "CHEST": 6(
				                [
				                    "[705:12->705:17)",
				                    "[705:20->705:21)",
				                ],
				            ),
				            "COFFEE": 7(
				                [
				                    "[706:12->706:18)",
				                    "[706:21->706:22)",
				                ],
				            ),
				            "CUP": 3(
				                [
				                    "[702:12->702:15)",
				                    "[702:18->702:19)",
				                ],
				            ),
				            "NITROWEEN_STANDARD": 12(
				                [
				                    "[711:12->711:30)",
				                    "[711:33->711:35)",
				                ],
				            ),
				            "SEASONAL_CAKE": 9(
				                [
				                    "[708:12->708:25)",
				                    "[708:28->708:29)",
				                ],
				            ),
				            "SEASONAL_CHEST": 10(
				                [
				                    "[709:12->709:26)",
				                    "[709:29->709:31)",
				                ],
				            ),
				            "SEASONAL_COFFEE": 11(
				                [
				                    "[710:12->710:27)",
				                    "[710:30->710:32)",
				                ],
				            ),
				            "SEASONAL_STANDARD_BOX": 8(
				                [
				                    "[707:12->707:33)",
				                    "[707:36->707:37)",
				                ],
				            ),
				            "SNOWGLOBE": 1(
				                [
				                    "[700:19->700:28)",
				                    "[700:31->700:32)",
				                ],
				            ),
				            "STANDARD_BOX": 4(
				                [
				                    "[703:12->703:24)",
				                    "[703:27->703:28)",
				                ],
				            ),
				            "SYM_CJS_DEFAULT": [
				                "[17:8->17:10)",
				                "[699:8->699:10)",
				            ],
				        },
				    ),
				    "Si": ExportMap(
				        {
				            "GUILD": "590663762298667008"(
				                [
				                    "[153:10->153:15)",
				                    "[153:18->153:38)",
				                ],
				            ),
				            "LEGACY": "521842865731534868"(
				                [
				                    "[154:10->154:16)",
				                    "[154:19->154:39)",
				                ],
				            ),
				            "NONE": "628379670982688768"(
				                [
				                    "[149:17->149:21)",
				                    "[149:24->149:44)",
				                ],
				            ),
				            "TIER_0": "978380684370378762"(
				                [
				                    "[150:10->150:16)",
				                    "[150:19->150:39)",
				                ],
				            ),
				            "TIER_1": "521846918637420545"(
				                [
				                    "[151:10->151:16)",
				                    "[151:19->151:39)",
				                ],
				            ),
				            "TIER_2": "521847234246082599"(
				                [
				                    "[152:10->152:16)",
				                    "[152:19->152:39)",
				                ],
				            ),
				            "SYM_CJS_DEFAULT": [
				                "[44:8->44:10)",
				                "[148:8->148:9)",
				            ],
				        },
				    ),
				}
				"#);
		}
	}
	mod e_exports {
		use super::*;
		#[test]
		/// class names
		fn object_literal_exports() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/objLiteral.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "addButton": "addButton_f5cb44"(
			        [
			            "[7:8->7:17)",
			            "[7:19->7:37)",
			        ],
			    ),
			    "addButtonInner": "addButtonInner_f5cb44"(
			        [
			            "[8:8->8:22)",
			            "[8:24->8:47)",
			        ],
			    ),
			    "productListings": "productListings_f5cb44"(
			        [
			            "[6:8->6:23)",
			            "[6:25->6:49)",
			        ],
			    ),
			    "productListingsHeader": "productListingsHeader_f5cb44"(
			        [
			            "[5:8->5:29)",
			            "[5:31->5:61)",
			        ],
			    ),
			}
			"#);
		}
		#[test]
		fn single_string_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/string.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "SYM_CJS_DEFAULT": "/assets/b8deed70d3e4a9bd.svg"(
			        [
			            "[4:16->4:46)",
			        ],
			    ),
			}
			"#);
		}
		#[test]
		fn re_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/identReExport.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "SYM_CJS_DEFAULT": [
			        "[4:12->4:21)",
			    ],
			}
			"#);
		}
		#[test]
		fn exports_with_an_intermediate_var() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/ident.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "closeContainer": "closeContainer__2dea3"(
			        [
			            "[6:8->6:22)",
			            "[6:24->6:47)",
			        ],
			    ),
			    "closeIcon": "closeIcon__2dea3"(
			        [
			            "[7:8->7:17)",
			            "[7:19->7:37)",
			        ],
			    ),
			    "confirmationContainer": "confirmationContainer__2dea3"(
			        [
			            "[10:8->10:29)",
			            "[10:31->10:61)",
			        ],
			    ),
			    "confirmationSubtitle": "confirmationSubtitle__2dea3"(
			        [
			            "[13:8->13:28)",
			            "[13:30->13:59)",
			        ],
			    ),
			    "confirmationTitle": "confirmationTitle__2dea3"(
			        [
			            "[12:8->12:25)",
			            "[12:27->12:53)",
			        ],
			    ),
			    "headerContainer": "headerContainer__2dea3"(
			        [
			            "[5:8->5:23)",
			            "[5:25->5:49)",
			        ],
			    ),
			    "headerImage": "headerImage__2dea3"(
			        [
			            "[8:8->8:19)",
			            "[8:21->8:41)",
			        ],
			    ),
			    "headerImageContainer": "headerImageContainer__2dea3"(
			        [
			            "[9:8->9:28)",
			            "[9:30->9:59)",
			        ],
			    ),
			    "purchaseConfirmation": "purchaseConfirmation__2dea3 confirmationContainer__2dea3"(
			        [
			            "[11:8->11:28)",
			            "[11:30->11:88)",
			        ],
			    ),
			}
			"#);
		}
		#[test]
		fn function_expression() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/function.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "SYM_CJS_DEFAULT": [
			        "[9:16->9:28)",
			    ],
			}
			"#);
		}
		#[test]
		fn class_default_export() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/classExport.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "_dispatch": [
			        "[112:8->112:17)",
			    ],
			    "_dispatchWithDevtools": [
			        "[88:8->88:29)",
			    ],
			    "_dispatchWithLogging": [
			        "[91:8->91:28)",
			    ],
			    "addDependencies": [
			        "[150:8->150:23)",
			    ],
			    "addInterceptor": [
			        "[127:8->127:22)",
			    ],
			    "createToken": [
			        "[147:8->147:19)",
			    ],
			    "dispatch": [
			        "[38:8->38:16)",
			    ],
			    "dispatchForStoreTest": [
			        "[55:8->55:28)",
			    ],
			    "flushWaitQueue": [
			        "[60:8->60:22)",
			    ],
			    "isDispatching": [
			        "[35:8->35:21)",
			    ],
			    "register": [
			        "[144:8->144:16)",
			    ],
			    "subscribe": [
			        "[134:8->134:17)",
			    ],
			    "unsubscribe": [
			        "[139:8->139:19)",
			    ],
			    "wait": [
			        "[130:8->130:12)",
			    ],
			    "SYM_CJS_DEFAULT": [
			        "[34:10->34:11)",
			        "[153:8->153:19)",
			    ],
			}
			"#);
		}
		#[test]
		/// `parses_everything_else` from js
		fn ponyfill() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/e.exports/everythingElse.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "SYM_CJS_DEFAULT": [
			        "[5:16->5:44)",
			    ],
			}
			"#);
		}
	}
	mod exports {
		use super::*;
		#[test]
		fn pre_es6_class() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/exports/module.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "Deflate": [
			        "[101:6->101:13)",
			        "[18:13->18:14)",
			    ],
			    "deflate": [
			        "[102:6->102:13)",
			        "[49:13->49:14)",
			    ],
			    "deflateRaw": [
			        "[103:6->103:16)",
			        "[56:13->56:14)",
			    ],
			    "gzip": [
			        "[104:6->104:10)",
			        "[60:13->60:14)",
			    ],
			}
			"#);
		}
	}
	mod stores {
		use super::*;
	}
}
