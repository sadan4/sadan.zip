use std::mem;

use crate::{VencordAstParser, pass::dump_ast};
use insta::assert_ron_snapshot;
use itertools::Itertools as _;
use oxc::{allocator::Allocator, span::Span};
use smol_str::SmolStr;

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

#[test]
fn mixed_string_concatentaions_and_templates() {
	let code = include_str!("data/plugin11.tsx");
	let a = Allocator::new();
	let parser =
		VencordAstParser::try_new(&a, code, Some("data/plugin11.tsx")).unwrap();
	let patches = parser.patches(false).unwrap();
	assert_ron_snapshot!(patches);
}

#[test]
fn arrow_func_with_brace_body_and_single_return() {
	let code = include_str!("data/plugin12.tsx");
	let a = Allocator::new();
	let parser =
		VencordAstParser::try_new(&a, code, Some("data/plugin12.tsx")).unwrap();
	let patches = parser.patches(false).unwrap();
	assert_ron_snapshot!(patches);
}

mod self_reference_tests {
	use super::*;

	#[test]
	fn collects_names_and_all_spans() {
		// offset 0 keeps spans equal to byte indices for easy assertions.
		let refs = VencordAstParser::collect_self_references(
			"$self.a + $self.a + $self.b",
			0,
		);
		assert_eq!(refs.len(), 2);
		// `a` is referenced twice, `b` once.
		let a = &refs[&SmolStr::new("a")];
		assert_eq!(
			a,
			&vec![Span::new(6, 7), Span::new(16, 17)],
			"both `a` references, spanning just the identifier"
		);
		assert_eq!(refs[&SmolStr::new("b")], vec![Span::new(26, 27)]);
	}

	#[test]
	fn ignores_escapes_bare_and_computed() {
		// `$$self.x` is an escaped `$`, bare `$self` has no `.prop`, and
		// `$self["x"]` is computed access — none are references.
		let refs = VencordAstParser::collect_self_references(
			r#"$$self.x and $self and $self["x"]"#,
			0,
		);
		assert!(refs.is_empty(), "got {refs:?}");
	}

	#[test]
	fn captures_only_top_level_access() {
		// `$self.foo.bar` references `foo` only.
		let refs =
			VencordAstParser::collect_self_references("$self.foo.bar", 0);
		assert_eq!(refs.len(), 1);
		assert_eq!(refs[&SmolStr::new("foo")], vec![Span::new(6, 9)]);
	}

	#[test]
	fn offset_is_applied() {
		let refs = VencordAstParser::collect_self_references("$self.x", 10);
		assert_eq!(refs[&SmolStr::new("x")], vec![Span::new(16, 17)]);
	}

	const PLUGIN_SRC: &str = r#"import definePlugin from "@utils/types";
let foo;
export default definePlugin({
    name: "P",
    myMethod() {},
    patches: [{
        find: "abc",
        replacement: [{ match: /x/, replace: "$self.myMethod" }]
    }],
	foo,
});
"#;

	#[test]
	fn definition_resolves_to_plugin_key() {
		let alloc = oxc::allocator::Allocator::new();
		let parser =
			VencordAstParser::try_new(&alloc, PLUGIN_SRC, None).unwrap();

		// Offset inside the `myMethod` reference in the replacement string.
		let ref_off = (PLUGIN_SRC
			.find("$self.myMethod")
			.unwrap() + "$self.".len()) as u32;
		let def = parser
			.self_reference_definition(ref_off)
			.expect("expected a definition span");

		// It should point at the `myMethod` key in the definePlugin object.
		let key_start = PLUGIN_SRC
			.find("myMethod() {}")
			.unwrap() as u32;
		assert_eq!(
			def,
			Span::new(key_start, key_start + "myMethod".len() as u32)
		);
	}

	#[test]
	fn definition_resolves_in_func_and_template_replacements() {
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    myMethod() {},
    other() {},
    patches: [
        {
            find: "abc",
            replacement: { match: /x/, replace: (i) => `$self.myMethod(${i})` }
        },
        {
            find: "def",
            replacement: { match: /y/, replace: `prefix $self.other suffix` }
        }
    ],
});
"#;
		let alloc = oxc::allocator::Allocator::new();
		let parser = VencordAstParser::try_new(&alloc, SRC, None).unwrap();

		// `$self.myMethod` inside the arrow-function replacement.
		let func_reference =
			(SRC.find("$self.myMethod").unwrap() + "$self.".len()) as u32;
		let func_definition = parser
			.self_reference_definition(func_reference)
			.expect("expected a definition for the func replacement ref");
		let my_method = SRC.find("myMethod() {}").unwrap() as u32;
		assert_eq!(
			func_definition,
			Span::new(my_method, my_method + "myMethod".len() as u32)
		);

		// `$self.other` inside the template-literal replacement.
		let tmpl_reference =
			(SRC.find("$self.other").unwrap() + "$self.".len()) as u32;
		let tmpl_definition = parser
			.self_reference_definition(tmpl_reference)
			.expect("expected a definition for the template replacement ref");
		let other = SRC.find("other() {}").unwrap() as u32;
		assert_eq!(
			tmpl_definition,
			Span::new(other, other + "other".len() as u32)
		);
	}

	#[test]
	fn definition_none_off_reference_and_for_unknown_prop() {
		let alloc = oxc::allocator::Allocator::new();
		let parser =
			VencordAstParser::try_new(&alloc, PLUGIN_SRC, None).unwrap();

		// Cursor on the plugin `name` value, not a `$self` reference.
		let off = PLUGIN_SRC.find("\"P\"").unwrap() as u32 + 1;
		assert!(
			parser
				.self_reference_definition(off)
				.is_none()
		);

		// An unbound `$self` prop has no definition to jump to.
		const BAD: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "abc",
        replacement: { match: /x/, replace: "$self.nope" }
    }],
});
"#;
		let alloc = oxc::allocator::Allocator::new();
		let parser = VencordAstParser::try_new(&alloc, BAD, None).unwrap();
		let off = (BAD.find("$self.nope").unwrap() + "$self.".len()) as u32;
		assert!(
			parser
				.self_reference_definition(off)
				.is_none()
		);
	}
}
