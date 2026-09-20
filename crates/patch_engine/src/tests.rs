use insta::assert_snapshot;
use oxc::{allocator::Allocator, span::Span};
use pretty_printer::map_span;
use vencord_ast_parser::{Patch, VencordAstParser};

use crate::{
	Applied,
	ApplyEvent,
	ApplyOptions,
	SyntaxErrorReport,
	apply_patch,
	compile_patch_regexes,
};

/// A plugin whose only patch has `replacements` as its `replacement:` array.
fn plugin(replacements: &str) -> String {
	format!(
		r#"import definePlugin from "@utils/types";

export default definePlugin({{
    name: "TestPlugin",
    patches: [
        {{
            find: "needle",
            replacement: [{replacements}]
        }},
    ]
}});
"#
	)
}

/// The one patch `plugin(replacements)` declares, ready to apply.
fn patch_of(alloc: &Allocator, src: &str) -> Patch {
	let parser =
		VencordAstParser::try_new(alloc, src, Some("plugin.tsx")).unwrap();
	let mut patches = parser.patches(true).unwrap();
	assert_eq!(patches.len(), 1, "the fixture declares one patch");
	let mut patch = patches.swap_remove(0);
	compile_patch_regexes(std::iter::once(&mut patch));
	patch
}

fn apply(replacements: &str, src: &str, opts: &ApplyOptions<'_>) -> Applied {
	let plugin_src = plugin(replacements);
	let mut alloc = Allocator::new();
	let patch = patch_of(&alloc, &plugin_src);
	apply_patch(&mut alloc, &patch, src.to_owned(), opts)
}

#[test]
fn string_match_replaces_only_the_first_occurrence() {
	let applied = apply(
		r#"{ match: "needle", replace: "pin" }"#,
		"0,f(needle,needle)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "0,f(pin,needle)");
	// a non-global match that hits twice is worth saying out loud
	assert!(
		matches!(
			applied.events.as_slice(),
			[ApplyEvent::MatchAmbiguous { .. }]
		),
		"{:?}",
		applied.events
	);
}

#[test]
fn string_match_expands_dollar_escapes() {
	// `$&` is the whole match, `$$` a literal `$`
	let applied = apply(
		r#"{ match: "needle", replace: "$$($&)" }"#,
		"0,f(needle)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "0,f($(needle))");
}

#[test]
fn regex_match_expands_capture_groups() {
	let applied = apply(
		r#"{ match: /f\((\i)\)/, replace: "g($1,$1)" }"#,
		"0,f(needle)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "0,g(needle,needle)");
}

#[test]
fn self_expands_to_the_plugin() {
	let applied = apply(
		r#"{ match: "needle", replace: "$self.thing" }"#,
		"0,f(needle)",
		&ApplyOptions {
			plugin_name: Some("TestPlugin"),
			..ApplyOptions::default()
		},
	);
	assert_eq!(
		applied.src,
		r#"0,f(Vencord.Plugins.plugins["TestPlugin"].thing)"#
	);
}

#[test]
fn self_is_left_alone_without_a_plugin_name() {
	let applied = apply(
		r#"{ match: "needle", replace: "$self.thing" }"#,
		"0,f(needle)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "0,f($self.thing)");
}

#[test]
fn a_match_that_finds_nothing_is_reported_and_skipped() {
	let applied = apply(
		r#"{ match: "haystack", replace: "pin" }"#,
		"0,f(needle)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "0,f(needle)");
	assert!(
		matches!(
			applied.events.as_slice(),
			[ApplyEvent::MatchNotFound { .. }]
		),
		"{:?}",
		applied.events
	);
}

#[test]
fn a_broken_replacement_is_dropped_when_asked() {
	let replacements = r#"
		{ match: "needle", replace: "}}}" },
		{ match: "f(", replace: "g(" }
	"#;
	let applied = apply(
		replacements,
		"0,f(needle)",
		&ApplyOptions {
			keep_broken_replacements: false,
			..ApplyOptions::default()
		},
	);
	// the first replacement is thrown away, the second still runs
	assert_eq!(applied.src, "0,g(needle)");
	assert!(
		matches!(applied.events.as_slice(), [ApplyEvent::SyntaxError { .. }]),
		"{:?}",
		applied.events
	);
}

#[test]
fn a_broken_replacement_is_kept_by_default() {
	let applied = apply(
		r#"{ match: "needle", replace: "}}}" }"#,
		"0,f(needle)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "0,f(}}})");
	assert!(
		matches!(applied.events.as_slice(), [ApplyEvent::SyntaxError { .. }]),
		"{:?}",
		applied.events
	);
}

#[test]
fn a_syntax_error_can_be_reported_against_formatted_source() {
	let applied = apply(
		r#"{ match: "needle", replace: "}}}" }"#,
		"0,f(needle)",
		&ApplyOptions {
			syntax_errors: SyntaxErrorReport::Formatted {
				file_name: Some("42.js"),
			},
			..ApplyOptions::default()
		},
	);
	let [ApplyEvent::SyntaxError { cause, .. }] = applied.events.as_slice()
	else {
		panic!("{:?}", applied.events);
	};
	let source = cause
		.source
		.as_ref()
		.expect("the formatted module is attached");
	assert_eq!(source.file_name.as_deref(), Some("42.js"));
	// the formatted copy carries the replacement that broke it
	assert!(source.source_code.contains("}}}"), "{}", source.source_code);
}

#[test]
fn spans_survive_formatting() {
	let src = "0,f(needle)";
	let formatted = pretty_printer::format(src, 2).unwrap();
	let needle = Span::new(4, 10);
	assert_eq!(&src[needle.start as usize..needle.end as usize], "needle");
	let moved = map_span(&formatted.mappings, needle);
	assert_eq!(
		&formatted.code[moved.start as usize..moved.end as usize],
		"needle"
	);
}

/// The formatted module attached to the single [`ApplyEvent::SyntaxError`] in
/// `applied`, and the text each of the diagnostic's labels covers in it.
fn render_error(applied: &Applied) -> String {
	let [ApplyEvent::SyntaxError { cause, .. }] = applied.events.as_slice()
	else {
		panic!("{:?}", applied.events);
	};
	let handler = miette_ui::mk_test_handler();
	let mut buf = String::with_capacity(2048);
	handler
		.render_report(&mut buf, &**cause)
		.expect("Failed to render syntax error");
	buf
}

#[test]
fn labels_after_grow() {
	// minified, so formatting actually moves things and the mapping matters
	let applied = apply(
		r#"{ match: "x", replace: "yyyy" }"#,
		"let x=1;let yyyy=2",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "let yyyy=1;let yyyy=2");
	let err = render_error(&applied);
	assert_snapshot!(err, @"
	 x Identifier `yyyy` has already been declared
	  ,-[file.js:1:5]
	1 | let yyyy = 1;
	  :     ^^|^
	  :       `-- `yyyy` has already been declared here
	2 | let yyyy = 2
	  :     ^^|^
	  :       `-- It can not be redeclared here
	  `----
	");
}

#[test]
fn labels_after_shrink() {
	let applied = apply(
		r#"{ match: "qqqqqqqq", replace: "q" }"#,
		"let qqqqqqqq=1;let q=2",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "let q=1;let q=2");
	let err = render_error(&applied);
	assert_snapshot!(err, @"
	 x Identifier `q` has already been declared
	  ,-[file.js:1:5]
	1 | let q = 1;
	  :     |
	  :     `-- `q` has already been declared here
	2 | let q = 2
	  :     |
	  :     `-- It can not be redeclared here
	  `----
	");
}

#[test]
fn labels_inside_replacement() {
	let applied = apply(
		r#"{ match: "g(0)", replace: "let q=1,zz=2,q=3" }"#,
		"f(0);g(0);h(0)",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "f(0);let q=1,zz=2,q=3;h(0)");
	let err = render_error(&applied);
	assert_snapshot!(err, @"
	 x Identifier `q` has already been declared
	  ,-[file.js:2:5]
	1 | f(0);
	2 | let q=1,zz=2,q=3;
	  :     |        |
	  :     |        `-- It can not be redeclared here
	  :     `-- `q` has already been declared here
	3 | h(0)
	  `----
	");
}

#[test]
fn labels_inside_multiple_replacements() {
	let applied = apply(
		r#"{ match: /x|y/g, replace: "zzz" }"#,
		"let x=0;f(0);g(0);h(0);let y=0;",
		&ApplyOptions::default(),
	);
	assert_eq!(applied.src, "let zzz=0;f(0);g(0);h(0);let zzz=0;");
	let err = render_error(&applied);
	assert_snapshot!(err, @"
	 x Identifier `zzz` has already been declared
	  ,-[file.js:1:5]
	1 | let zzz = 0;
	  :     ^|^
	  :      `-- `zzz` has already been declared here
	2 | f(0);
	  `----
	  ,-[file.js:5:5]
	4 | h(0);
	5 | let zzz = 0;
	  :     ^|^
	  :      `-- It can not be redeclared here
	  `----
	");
}
