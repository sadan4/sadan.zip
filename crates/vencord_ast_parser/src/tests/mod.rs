use std::mem;

use crate::{VencordAstParser, pass::dump_ast};
use insta::assert_ron_snapshot;
use itertools::Itertools as _;
use oxc::allocator::Allocator;

macro_rules! dump_patches {
	($path:literal, $dump_code:literal) => {{
		let a = Allocator::new();
		let code = include_str!($path);
		let parser = VencordAstParser::try_new(&a, code, Some($path)).unwrap();
		if $dump_code {
			let code = dump_ast(&parser.prog);
			eprintln!("{code}");
		}
		parser.patches(true).unwrap()
	}};
	($path:literal) => {
		dump_patches!($path, false)
	};
	($path:literal, dbg_code) => {
		dump_patches!($path, true)
	};
}

#[test]
fn test_template_literal_replace() {
	let patches = dump_patches!("data/plugin3.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_simple_arrow_func() {
	let patches = dump_patches!("data/plugin4.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_concat_template() {
	let patches = dump_patches!("data/plugin1.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_concat_template_with_ident() {
	let patches = dump_patches!("data/plugin2.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_inline_big_int_literal_expr() {
	let patches = dump_patches!("data/plugin5.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
#[ignore = "todo"]
fn test_array_map_in_replace() {
	let patches = dump_patches!("data/plugin6.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_inline_string_raw() {
	let patches = dump_patches!("data/plugin7.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_inline_typescript_enums() {
	let patches = dump_patches!("data/plugin8.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn test_plugin_9() {
	let patches = dump_patches!("data/plugin9.tsx");
	assert_ron_snapshot!(patches);
}

#[test]
fn gets_plugin_name() {
	let a = Allocator::new();
	let code = include_str!("data/plugin1.tsx");
	let parser =
		VencordAstParser::try_new(&a, code, Some("data/plugin1.tsx")).unwrap();
	let plugin_name = parser.plugin_info().unwrap().name;
	assert_eq!(plugin_name, "Plugin1");
}

#[test]
fn gets_capture_group_ranges() {
	let a = Allocator::new();
	let code = include_str!("data/plugin10.tsx");
	let parser =
		VencordAstParser::try_new(&a, code, Some("data/plugin10.tsx")).unwrap();
	let patches = parser.patches(true).unwrap();
	let patch = &patches[0];
	let capture_group_ranges = patch.replacement[0]
		.match_
		.v
		.unwrap_regex_ref()
		.capture_spans
		.iter()
		.map(|span| &code[*span])
		.collect_vec();
	let replace_ranges = patch.replacement[0]
		.replace
		.used_replace_capture_spans
		.iter()
		.flatten()
		.map(|span| &code[*span])
		.collect_vec();
	assert_ron_snapshot!(capture_group_ranges, @r#"
	[
	  "(\\i)",
	  "(.{1,500}\\i)",
	  "(\\i)",
	  "(Math\\.max\\(\\d+?,\\i(?:-\\i\\.length){2}\\))",
	]
	"#);
	assert_ron_snapshot!(replace_ranges, @r#"
	[
	  "$2",
	  "$3",
	  "$3",
	  "$4",
	]
	"#);
}

#[test]
#[expect(clippy::too_many_lines)]
fn gets_plugin_meta() {
	let a = Allocator::new();
	let plugin8 = include_str!("data/plugin8.tsx");
	let plugin9 = include_str!("data/plugin9.tsx");
	let plugin10 = include_str!("data/plugin10.tsx");
	let mut plugin_infos = Vec::new();
	// we need to have these in a separate vec so we can sort them
	let mut tlpk = Vec::new();
	for code in [plugin8, plugin9, plugin10] {
		let parser =
			VencordAstParser::try_new(&a, code, Some("data/pluginX.tsx"))
				.unwrap();
		let mut info = parser.plugin_info().unwrap();
		tlpk.push(
			mem::take(&mut info.top_level_plugin_keys)
				.into_iter()
				.sorted()
				.collect_vec(),
		);

		plugin_infos.push(info);
	}
	assert_ron_snapshot!(tlpk, @r#"
	[
	  [
	    ("name", {
	      "start": 534,
	      "end": 538,
	    }),
	    ("patches", {
	      "start": 556,
	      "end": 563,
	    }),
	  ],
	  [
	    ("authors", {
	      "start": 315,
	      "end": 322,
	    }),
	    ("description", {
	      "start": 207,
	      "end": 218,
	    }),
	    ("name", {
	      "start": 177,
	      "end": 181,
	    }),
	    ("patches", {
	      "start": 358,
	      "end": 365,
	    }),
	    ("setShift", {
	      "start": 1182,
	      "end": 1190,
	    }),
	    ("shouldTransition", {
	      "start": 1081,
	      "end": 1097,
	    }),
	  ],
	  [
	    ("authors", {
	      "start": 468,
	      "end": 475,
	    }),
	    ("description", {
	      "start": 504,
	      "end": 515,
	    }),
	    ("name", {
	      "start": 436,
	      "end": 440,
	    }),
	    ("patches", {
	      "start": 621,
	      "end": 628,
	    }),
	    ("sortEmojis", {
	      "start": 1834,
	      "end": 1844,
	    }),
	    ("tags", {
	      "start": 582,
	      "end": 586,
	    }),
	  ],
	]
	"#);
	assert_ron_snapshot!(plugin_infos, @r#"
	[
	  PluginInfo(
	    name: "Plugin8",
	    description: None,
	    devs: None,
	    top_level_plugin_keys: {},
	    span: {
	      "start": 528,
	      "end": 3118,
	    },
	  ),
	  PluginInfo(
	    name: "NoFollowForwards",
	    description: Some("After forwarding a single message, don\'t jump to it. Hold shift to ignore this behavior"),
	    devs: Some([
	      PluginDev(
	        dev: Reference(
	          key: "Sqaaakoi",
	          obj: "Devs",
	        ),
	        span: {
	          "start": 325,
	          "end": 338,
	        },
	      ),
	      PluginDev(
	        dev: Reference(
	          key: "sadan",
	          obj: "Devs",
	        ),
	        span: {
	          "start": 340,
	          "end": 350,
	        },
	      ),
	    ]),
	    top_level_plugin_keys: {},
	    span: {
	      "start": 171,
	      "end": 1270,
	    },
	  ),
	  PluginInfo(
	    name: "FavoriteEmojiFirst",
	    description: Some("Puts your favorite emoji first in the emoji autocomplete."),
	    devs: Some([
	      PluginDev(
	        dev: Reference(
	          key: "Aria",
	          obj: "Devs",
	        ),
	        span: {
	          "start": 478,
	          "end": 487,
	        },
	      ),
	      PluginDev(
	        dev: Reference(
	          key: "Ven",
	          obj: "Devs",
	        ),
	        span: {
	          "start": 489,
	          "end": 497,
	        },
	      ),
	    ]),
	    top_level_plugin_keys: {},
	    span: {
	      "start": 430,
	      "end": 2578,
	    },
	  ),
	]
	"#);
}
