#![allow(clippy::unreadable_literal, clippy::needless_raw_string_hashes)]
mod util;

use insta::{assert_debug_snapshot, assert_snapshot};
use macros::cache_test;

use util::Bundle;

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
	        range: "[17:26->17:27) J",
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
	            range: "[17:18->17:19) H",
	        },
	        ReferenceDumper {
	            id: ModuleId(
	                111111,
	            ),
	            range: "[17:40->17:41) H",
	        },
	        ReferenceDumper {
	            id: ModuleId(
	                999999,
	            ),
	            range: "[14:41->14:42) H",
	        },
	    ],
	)
	"#);
}

/// finds all uses of a default e.exports where the exports
/// are assigned to the default export first
mod e_exports_default {
	use crate::util::dbg_export_map;

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
		        111112,
		    ],
		    lazy: [],
		}
		");
		let map = dbg_export_map(&parser);
		assert_snapshot!(map, @r#"
		{
		    "bar": [
		        "[8:8->8:11) bar",
		        "[8:11->8:14) () ",
		    ],
		    "baz": 2(
		        [
		            "[11:8->11:11) baz",
		            "[11:13->11:14) 2",
		        ],
		    ),
		    "foo": [
		        "[5:8->5:11) foo",
		        "[5:13->5:24) function() ",
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
		        range: "[33:28->33:31) foo",
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
		        range: "[34:28->34:31) bar",
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
		        range: "[35:28->35:31) baz",
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
	        range: "[14:52->14:53) H",
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
			        range: "[20:25->20:29) FOO1",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[45:17->45:21) FOO1",
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
			        range: "[20:23->20:24) E",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[43:23->43:24) E",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[45:15->45:16) E",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[45:26->45:27) E",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            222222,
			        ),
			        range: "[20:23->20:24) E",
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
			        range: "[20:36->20:40) BAR2",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[46:28->46:32) BAR2",
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
			        range: "[20:34->20:35) F",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[43:29->43:30) F",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[46:15->46:16) F",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            111111,
			        ),
			        range: "[46:26->46:27) F",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            222222,
			        ),
			        range: "[20:32->20:33) F",
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
			        range: "[17:13->17:15) _g",
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
			            range: "[12:13->12:15) _u",
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
					        range: "[21:8->21:18) style1Enum",
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
					        range: "[21:8->21:18) style1Enum",
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
					        range: "[22:24->22:30) \"FOO1\"",
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
					        range: "[26:8->26:18) style2Enum",
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
					        range: "[26:8->26:18) style2Enum",
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
					        range: "[28:19->28:20) 2",
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
		        range: "[8:8->8:11) foo",
		    },
		]
		"#);
	}
}
mod hover_text {
	use crate::util::dbg_hover;

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
		    "[38:11->38:13) ZP",
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
		    "[32:23->32:25) ZP",
		)
		"#);
		assert_debug_snapshot!(hov, @r#"
		(
		    "MyTestingStore",
		    "[15:23->15:25) ZP",
		)
		"#);
	}
}

mod references {
	use super::*;
	mod re_exports {
		use super::*;
		#[cache_test(sub_dir = "re_export")]
		fn handles_re_export(b: &Bundle) {
			let parser = b.parse(6151);
			let locs = b.dbg_gen_refs(&parser, 6, 8).unwrap();
			assert_debug_snapshot!(locs, @r#"
			[
			    ReferenceDumper {
			        id: ModuleId(
			            67956,
			        ),
			        range: "[13:18->13:20) v7",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            637141,
			        ),
			        range: "[11:17->11:18) v",
			    },
			    ReferenceDumper {
			        id: ModuleId(
			            944355,
			        ),
			        range: "[5:20->5:21) v",
			    },
			]
			"#);
		}
	}
}
