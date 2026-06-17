use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

const TEST_FUNCTION_NAME: &str = "cache_test_impl__";

pub fn cache_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut tfn = parse_macro_input!(item as ItemFn);
	let args = &tfn.sig.inputs;
	if args.len() != 1 {
		return syn::Error::new_spanned(
			args,
			"Expected exactly one argument of type &Bundle",
		)
		.into_compile_error()
		.into();
	}
	let orig_fn_name = tfn.sig.ident;
	tfn.sig.ident = syn::Ident::new(TEST_FUNCTION_NAME, orig_fn_name.span());
	let new_fn_name = &tfn.sig.ident;
	quote! {
		#[core::prelude::v1::test]
		fn #orig_fn_name() {
			#tfn
			let alloc = ::oxc::allocator::Allocator::new();
			let (b, parsers) = crate::Bundle::try_new(&alloc).unwrap();
			b.bind_plugins(parsers);
			#new_fn_name(&b);
		}
	}
	.into()
}
