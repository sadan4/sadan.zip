use miette::{GraphicalReportHandler, GraphicalTheme};
use oxc_span::Span;

use crate::{ParserDiagnostic, err};

fn render(diag: ParserDiagnostic, source: &str) -> String {
	let mut out = String::new();
	GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
		.with_width(80)
		.render_report(&mut out, &diag.with_local_source(source, "test.js"))
		.unwrap();
	out
}

/// one minified line, `len` bytes long, with `needle` at `at`
fn minified(len: usize, at: usize, needle: &str) -> String {
	let mut src = "a".repeat(len);
	src.replace_range(at..at + needle.len(), needle);
	src
}

#[test]
fn span_far_into_long_line() {
	let src = minified(100_000, 90_000, "needle");
	let out = render(err(&Span::new(90_000, 90_006), "here"), &src);
	assert!(out.contains("[test.js:1:90001]"), "{out}");
	assert!(out.contains("needle"), "{out}");
	assert!(out.len() < 2_000, "{out}");
}

#[test]
fn span_longer_than_window() {
	let src = minified(100_000, 70_000, "needle");
	let out = render(err(&Span::new(70_000, 90_000), "here"), &src);
	assert!(out.contains("[test.js:1:70001]"), "{out}");
	assert!(out.len() < 4_000, "{out}");
}

#[test]
fn span_crosses_into_long_line() {
	let src = format!("short\n{}", minified(100_000, 90_000, "needle"));
	let out = render(err(&Span::new(2, 90_010), "here"), &src);
	assert!(out.contains("[test.js:1:3]"), "{out}");
	assert!(out.len() < 4_000, "{out}");
}

#[test]
fn labels_far_apart_on_one_long_line() {
	let src = minified(100_000, 80_000, "needle");
	let diag = ParserDiagnostic {
		msg: "two".into(),
		labels: vec![
			(Span::new(10, 12), "first".into()),
			(Span::new(80_000, 80_006), "second".into()),
		],
		..Default::default()
	};
	let out = render(diag, &src);
	assert!(out.contains("first"), "{out}");
	assert!(out.contains("second"), "{out}");
}

#[test]
fn multibyte_chars_around_window_edges() {
	let src = "é".repeat(50_000);
	// offset 80_001 is inside a char, and so are the window edges
	let out = render(err(&Span::new(80_000, 80_002), "here"), &src);
	assert!(out.contains("here"), "{out}");
}

#[test]
fn span_at_end_of_long_line() {
	let src = "a".repeat(100_000);
	let out = render(err(&Span::new(100_000, 100_000), "eof"), &src);
	assert!(out.contains("eof"), "{out}");
}

#[test]
fn short_lines_keep_context() {
	let src = "one\ntwo\nthree\nfour\nfive";
	let out = render(err(&Span::new(8, 13), "here"), src);
	assert!(out.contains("two"), "{out}");
	assert!(out.contains("four"), "{out}");
}

#[test]
fn long_line_gutter_uses_span_line() {
	let src = format!("one\ntwo\n{}", minified(100_000, 90_000, "needle"));
	let out = render(err(&Span::new(90_008, 90_014), "here"), &src);
	assert!(out.contains("[test.js:3:90001]"), "{out}");
	assert!(out.contains(" 3 │"), "{out}");
	assert!(!out.contains(" 2 │"), "{out}");
}
