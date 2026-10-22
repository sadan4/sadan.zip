#![allow(clippy::unreadable_literal, clippy::needless_raw_string_hashes)]

use insta::{assert_debug_snapshot};
use macros::cache_test;
use webpack_ast_parser::css::ClassName;

use crate::util::Bundle;
#[allow(dead_code, reason = "shared with other integration tests")]
mod util;

#[cache_test(sub_dir = "composing_modules")]
async fn test_css_module_compositoin(b: &Bundle) {
	let p = b.parse(8936);
	let p = p.parser();
	let m = p.as_css_module().await.unwrap();
	let name = m.class_names.into_iter().find_map(
		|ClassName {
		     exported_as,
		     value,
		     module: _,
		     span: _,
		 }| { (exported_as == "XG").then_some(value) },
	);
	assert_debug_snapshot!(name, @r#"
	Some(
	    "scroller_d7bc5d popover_d6f39b",
	)
	"#);
}
