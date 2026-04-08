mod test_log;

use proc_macro::TokenStream;

use crate::test_log::impl_test_macro;

// Documented in `test-log` crate's re-export.
#[allow(missing_docs)]
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
	impl_test_macro(attr, item)
}
