use ast_parser::parse_with_tokens;
use insta::assert_debug_snapshot;
use oxc::{allocator::Allocator, span::SourceType};

use crate::parse_;

#[test]
#[ignore = "only for local testing rn"]
fn doesnt_generate_bad_finds() {
	let alloc = Allocator::new();
	let parser = parse_!(alloc, "test_data/wp/rawModule.js");
	let finds = parser.dbg_finds();
	assert_debug_snapshot!(finds, @"");
}
