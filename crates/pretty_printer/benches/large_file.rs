use std::fmt::Write as _;

use criterion::{Criterion, criterion_group, criterion_main};
use pretty_printer::FormattedContent;

const LARGE_FILE: &str =
	include_str!("../../webpack_chunk_parser/src/test_data/fullWeb.js");

fn format_large_file() -> FormattedContent {
	pretty_printer::format(LARGE_FILE, 0).unwrap()
}

/// Build a many-line, valid-JS input. `LARGE_FILE` is 20MB but only ~29
/// lines (minified), so it never exercises `line_of_pos` with a large line
/// count. This input does: one statement per line.
fn many_line_src(lines: usize) -> String {
	let mut s = String::new();
	for i in 0..lines {
		writeln!(s, "const v{i} = {i} + foo(bar, baz) * qux;").unwrap();
	}
	s
}

fn bench_large_file(c: &mut Criterion) {
	c.bench_function("format_large_file", |b| b.iter(format_large_file));

	let src = many_line_src(8_000);
	c.bench_function("format_many_line_file", |b| {
		b.iter(|| pretty_printer::format(&src, 0).unwrap());
	});
}

criterion_group! {
	name = benches;
	config = Criterion::default();
	targets = bench_large_file
}
criterion_main!(benches);
