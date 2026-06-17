use criterion::{Criterion, criterion_group, criterion_main};
use pretty_printer::FormattedContent;

const LARGE_FILE: &str =
	include_str!("../../webpack_chunk_parser/src/test_data/fullWeb.js");

fn format_large_file() -> FormattedContent {
	pretty_printer::format(LARGE_FILE, 0).unwrap()
}

fn bench_large_file(c: &mut Criterion) {
	c.bench_function("format_large_file", |b| b.iter(format_large_file));
}

criterion_group! {
	name = benches;
	config = Criterion::default();
	targets = bench_large_file
}
criterion_main!(benches);
