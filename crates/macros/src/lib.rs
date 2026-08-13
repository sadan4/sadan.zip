mod cache_test;
mod command;
mod test_log;

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

#[expect(missing_docs)]
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
	command::command(&attr.into(), item.into())
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

#[expect(missing_docs)]
#[proc_macro_attribute]
pub fn executor(attr: TokenStream, item: TokenStream) -> TokenStream {
	command::executor(attr.into(), item.into())
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

#[expect(missing_docs)]
#[proc_macro_derive(SlashArgs)]
pub fn slash_args(item: TokenStream) -> TokenStream {
	command::slash_args_derive(item.into())
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

#[expect(missing_docs)]
#[proc_macro_derive(SlashChoices)]
pub fn slash_choices(item: TokenStream) -> TokenStream {
	command::slash_choices_derive(item.into())
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}
