mod test_log;
mod cache_test;

use proc_macro::TokenStream;

use crate::test_log::impl_test_macro;

// Documented in `test-log` crate's re-export.
#[expect(missing_docs)]
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
	impl_test_macro(attr, item)
}

#[expect(missing_docs)]
#[proc_macro_attribute]
pub fn cache_test(attr: TokenStream, item: TokenStream) -> TokenStream {
	cache_test::cache_test(attr, item)
}
