use std::{hint::black_box, time::Instant};

const LARGE_FILE: &str =
	include_str!("../../webpack_chunk_parser/src/test_data/fullWeb.js");

fn main() {
	do_format();
}

#[inline(never)]
fn do_format() {
	dbg!(cfg!(debug_assertions));
	println!("formatting a {} byte file", LARGE_FILE.len());
	let start = Instant::now();
	let res = pretty_printer::format(LARGE_FILE, 0).unwrap();
	let duration = start.elapsed();
	println!("Formatted large file in {duration:?}");
	black_box(res);
}
