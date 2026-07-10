use super::{test, *};
use std::{
	cmp::Reverse,
	fmt::{self, Debug},
};

use oxc::{allocator::Allocator, parser::Token, span::Span};

use crate::{WebpackAstParser, parse_};

#[derive(Copy, Clone)]
struct FindDumper<'ast>(u32, Span, &'ast str);

impl<'ast> FindDumper<'ast> {
	fn new(score: u32, tokens: &[Token], source: &'ast str) -> Self {
		assert!(!tokens.is_empty(), "tokens must not be empty");
		for [t1, t2] in tokens.iter().array_windows() {
			assert!(
				t1.end() <= t2.start(),
				"tokens must be in source order. t1: {t1:#?}, t2: {t2:#?}, t1 src: {}, t2 src: {}, combined source: {}",
				&source[t1.span()],
				&source[t2.span()],
				&source[Span::new(t1.start(), t2.end())]
			);
		}
		let start = tokens[0].start();
		let end = tokens.last().unwrap().end();
		Self(score, Span::new(start, end), source)
	}
}

impl Debug for FindDumper<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Find")
			.field("score", &self.0)
			.field("find", &&self.2[self.1])
			.finish()
	}
}

impl WebpackAstParser<'_> {
	fn dbg_finds(&self) -> Vec<FindDumper<'_>> {
		let mut finds = self.generate_finds();
		finds.sort_unstable_by_key(|f| Reverse(f.score));
		finds
			.into_iter()
			.map(|ts| FindDumper::new(ts.score, &ts.tokens, self.source))
			.collect()
	}
}

#[test]
fn doesnt_crash() {
	let alloc = Allocator::new();
	let parser = parse_!(alloc, "test_data/wp/rawModule.js");
	let finds = parser.dbg_finds();
	_ = &finds;
}
