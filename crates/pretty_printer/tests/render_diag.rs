use oxc::span::Span;
use parser_diag::err;
use pretty_printer::{format, render_diag};

const MINIFIED: &str =
	"function f(a){var b=a+1;return b}function g(){return 2}";

fn diag_span() -> Span {
	let start = MINIFIED.find("return b").unwrap() as u32;
	Span::new(start, start + "return b".len() as u32)
}

#[test]
fn map_pos_lands_on_the_same_token_in_the_formatted_source() {
	let fmt = format(MINIFIED, 4).unwrap();
	let original = MINIFIED.find("return b").unwrap() as u32;

	let formatted = fmt.map_pos(original) as usize;

	assert!(
		fmt.code[formatted..].starts_with("return b"),
		"expected `return b` at {formatted}, got: {:?}",
		&fmt.code[formatted..(formatted + 16).min(fmt.code.len())]
	);
}

#[test]
fn remapped_labels_point_into_the_formatted_source() {
	let fmt = format(MINIFIED, 4).unwrap();
	let diag = err(&diag_span(), "cannot return b here")
		.remap_spans(|pos| fmt.map_pos(pos));

	let (span, _) = diag.labels[0];

	assert_ne!(span, diag_span(), "the label was not remapped");
	// the mapped end can land just past a newline the formatter inserted, so
	// the label covers the token but is not required to end exactly on it
	assert!(
		fmt.code[span.start as usize..span.end as usize]
			.starts_with("return b"),
		"label covers {:?}",
		&fmt.code[span.start as usize..span.end as usize]
	);
}

#[test]
fn render_diag_reports_the_message() {
	let rendered = render_diag(
		err(&diag_span(), "cannot return b here"),
		MINIFIED,
		"12345.js",
	);

	assert!(rendered.contains("cannot return b here"), "{rendered}");
}
