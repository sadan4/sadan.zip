#![allow(clippy::unreadable_literal, clippy::too_many_lines)]
use super::*;
use ast_parser::span_line_and_column;
use insta::assert_debug_snapshot;
use itertools::Itertools;
use macros::test;
use oxc::span::{Atom, Span};
use std::fmt::{self, Debug};

macro_rules! parse {
	($alloc:expr, $source:literal) => {{
		let source = include_str!($source);
		WebpackAstParser::try_new(&$alloc, source).unwrap()
	}};
}

#[derive(Copy, Clone)]
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
	fn dbg_uses_of_import<'a>(
		&'a self,
		module_id: ModuleId,
		export_names: &[ExportMapKey],
	) -> Vec<SpanDumper<'a>> {
		self.get_uses_of_import(module_id, export_names)
			.into_iter()
			.map(|span| SpanDumper(span, self.source))
			.collect()
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
	use macros::test;

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
		use macros::test;
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
		use macros::test;
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
		use macros::test;
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
		use macros::test;
		#[test]
		fn normal_store() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/stores/store1.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "Z": EnablePublicGuildUpsellNoticeStore(
			        {
			            "initialize": [
			                "[11:8->11:18)",
			            ],
			            "isVisible": [
			                "[18:8->18:17)",
			            ],
			            "SYM_CJS_DEFAULT": [
			                "[4:8->4:9)",
			                "[32:16->32:17)",
			                "[10:10->10:11)",
			            ],
			        },
			    ),
			}
			"#);
		}
		#[test]
		fn ctor_with_no_args() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/stores/store2.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "ASSISTANT_WUMPUS_VOICE_USER": "47835198259242069"(
			        [
			            "[6:8->6:35)",
			            "[39:12->39:31)",
			        ],
			    ),
			    "default": UserStore(
			        {
			            "filter": [
			                "[260:8->260:14)",
			            ],
			            "findByTag": [
			                "[253:8->253:17)",
			            ],
			            "forEach": [
			                "[248:8->248:15)",
			            ],
			            "getCurrentUser": [
			                "[270:8->270:22)",
			            ],
			            "getUser": [
			                "[241:8->241:15)",
			            ],
			            "getUserStoreVersion": 0(
			                [
			                    "[38:12->38:13)",
			                ],
			            ),
			            "getUsers": [
			                "[245:8->245:16)",
			            ],
			            "handleLoadCache": [
			                "[224:8->224:23)",
			            ],
			            "initialize": [
			                "[212:8->212:18)",
			            ],
			            "takeSnapshot": [
			                "[215:8->215:20)",
			            ],
			            "SYM_CJS_DEFAULT": [
			                "[7:8->7:15)",
			                "[286:17->286:19)",
			                "[211:10->211:12)",
			                "[273:8->273:19)",
			            ],
			        },
			    ),
			    "mergeUser": [
			        "[8:8->8:17)",
			        "[118:13->118:14)",
			    ],
			}
			"#);
		}
		#[test]
		fn no_initialize_method() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/stores/store3.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "Z": GuildStore(
			        {
			            "getAllGuildsRoles": [
			                "[218:8->218:25)",
			            ],
			            "getGeoRestrictedGuilds": [
			                "[53:12->53:14)",
			            ],
			            "getGuild": [
			                "[199:8->199:16)",
			            ],
			            "getGuildCount": [
			                "[4:8->4:9)",
			            ],
			            "getGuildIds": [
			                "[206:8->206:19)",
			            ],
			            "getGuilds": [
			                "[203:8->203:17)",
			            ],
			            "getRole": [
			                "[225:8->225:15)",
			            ],
			            "getRoles": [
			                "[221:8->221:16)",
			            ],
			            "isLoaded": [
			                "[52:12->52:14)",
			            ],
			            "SYM_CJS_DEFAULT": [
			                "[6:8->6:9)",
			                "[231:16->231:17)",
			                "[198:10->198:11)",
			            ],
			        },
			    ),
			}
			"#);
		}

		#[test]
		fn with_getters() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/stores/getter.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "Z": SoundboardOverlayStore(
			        {
			            "enabled": [
			                "[7:12->7:14)",
			            ],
			            "keepOpen": [
			                "[8:12->8:14)",
			            ],
			            "SYM_CJS_DEFAULT": [
			                "[4:8->4:9)",
			                "[24:16->24:17)",
			                "[9:10->9:11)",
			            ],
			        },
			    ),
			}
			"#);
		}

		#[test]
		// TODO: libdiscore stores use `define(this, key, prop)` in the constructor a lot
		// which we don't parse
		fn using_libdiscore() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/stores/store-libdiscore-1.js");
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "A": GuildStore(
			        {
			            "getGuildCount": [
			                "[71:8->71:21)",
			            ],
			            "stateWrapper": [
			                "[68:8->68:20)",
			            ],
			            "SYM_CJS_DEFAULT": [
			                "[4:8->4:9)",
			                "[88:16->88:17)",
			                "[67:10->67:11)",
			                "[74:8->74:19)",
			            ],
			        },
			    ),
			}
			"#);
		}

		#[test]
		fn with_static_properties() {
			let alloc = Allocator::new();
			let p = parse!(
				alloc,
				"test_data/wp/stores/store-static-displayName.js"
			);
			let map = p.dbg_export_map();
			assert_debug_snapshot!(map, @r#"
			{
			    "ASSISTANT_WUMPUS_VOICE_USER": "47835198259242069"(
			        [
			            "[6:8->6:35)",
			            "[35:12->35:31)",
			        ],
			    ),
			    "default": "UserStore"(
			        {
			            "LATEST_SNAPSHOT_VERSION": 1(
			                [
			                    "[594:41->594:42)",
			                ],
			            ),
			            "displayName": "UserStore"(
			                [
			                    "[593:29->593:40)",
			                ],
			            ),
			            "filter": [
			                "[710:8->710:14)",
			            ],
			            "findByTag": [
			                "[703:8->703:17)",
			            ],
			            "forEach": [
			                "[698:8->698:15)",
			            ],
			            "getCurrentUser": [
			                "[720:8->720:22)",
			            ],
			            "getUser": [
			                "[691:8->691:15)",
			            ],
			            "getUserStoreVersion": 0(
			                [
			                    "[34:12->34:13)",
			                ],
			            ),
			            "getUsers": [
			                "[695:8->695:16)",
			            ],
			            "handleLoadCache": [
			                "[676:8->676:23)",
			            ],
			            "initialize": [
			                "[664:8->664:18)",
			            ],
			            "takeSnapshot": [
			                "[667:8->667:20)",
			            ],
			            "SYM_CJS_DEFAULT": [
			                "[7:8->7:15)",
			                "[724:17->724:19)",
			                "[592:10->592:12)",
			                "[595:8->595:19)",
			            ],
			        },
			    ),
			    "mergeUser": [
			        "[8:8->8:17)",
			        "[128:13->128:14)",
			    ],
			    "transformUser": [
			        "[9:8->9:21)",
			        "[71:13->71:14)",
			    ],
			    "users": [
			        "[10:8->10:13)",
			        "[10:15->10:21)",
			    ],
			}
			"#);
		}

		#[test]
		#[ignore = "never seen example of this"]
		fn exported_via_module_exports() {
			unimplemented!();
		}

		#[test]
		#[ignore = "never seen example of this"]
		fn exported_via_exports() {
			unimplemented!();
		}
	}
}

mod import_parsing {
	use super::*;
	use macros::test;

	fn k(s: &'static str) -> ExportMapKey {
		SmolStr::new_static(s).into()
	}

	#[test]
	fn only_reexported_export() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/reExport.js");
		let uses = p.dbg_uses_of_import(ModuleId(999001), &[k("foo")]);
		assert_debug_snapshot!(uses, @r#"
		[
		    "[5:21->5:24)",
		]
		"#);
	}
	#[test]
	fn reexport_with_other_uses() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/reExport.js");
		let uses = p.dbg_uses_of_import(ModuleId(999001), &[k("bar")]);
		assert_debug_snapshot!(uses, @r#"
		[
		    "[6:22->6:25)",
		    "[10:18->10:21)",
		]
		"#);
	}
	#[test]
	fn empty_when_no_uses() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/reExport.js");
		let uses = p.dbg_uses_of_import(ModuleId(999001), &[k("baz")]);
		assert_debug_snapshot!(uses, @"[]");
	}
	#[test]
	fn empty_when_not_imported() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/reExport.js");
		let uses = p.dbg_uses_of_import(ModuleId(999003), &[k("foo")]);
		assert_debug_snapshot!(uses, @"[]");
	}

	#[test]
	fn empty_when_no_uses_2() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/indirectCall.js");
		let uses = p.dbg_uses_of_import(ModuleId(999002), &[k("bar")]);
		assert_debug_snapshot!(uses, @"[]");
	}
	#[test]
	fn empty_when_not_imported_2() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/indirectCall.js");
		let uses = p.dbg_uses_of_import(ModuleId(999004), &[k("foo")]);
		assert_debug_snapshot!(uses, @"[]");
	}
	#[test]
	fn indirect_call() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/indirectCall.js");
		let uses = p.dbg_uses_of_import(ModuleId(999002), &[k("foo")]);
		assert_debug_snapshot!(uses, @r#"
		[
		    "[9:22->9:25)",
		]
		"#);
	}
	#[test]
	fn direct_call() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/directCall.js");
		let uses = p.dbg_uses_of_import(ModuleId(999003), &[k("foo3")]);
		assert_debug_snapshot!(uses, @r#"
		[
		    "[8:29->8:33)",
		]
		"#);
	}

	#[test]
	fn none_when_wreq_unused() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/directCall.js");
		let uses = p.get_uses_of_import(ModuleId(0), &[]);
		assert_eq!(uses, vec![]);
	}

	#[test]
	fn node_default_exports() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/nodeModule.js");
		let uses =
			p.dbg_uses_of_import(ModuleId(999005), &[ExportMapKey::Default]);
		assert_debug_snapshot!(uses, @r#"
		[
		    "[15:8->15:12)",
		    "[20:15->20:19)",
		]
		"#);
	}

	#[test]
	fn node_named_exports() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/imports/nodeModule.js");
		let uses = p.dbg_uses_of_import(ModuleId(999005), &[k("qux")]);
		assert_debug_snapshot!(uses, @r#"
		[
		    "[16:20->16:23)",
		    "[19:13->19:16)",
		]
		"#);
	}
}
