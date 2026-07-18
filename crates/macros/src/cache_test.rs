use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

const TEST_FUNCTION_NAME: &str = "cache_test_impl__";

pub fn cache_test(attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut sub_dir: Option<LitStr> = None;
	let parser = syn::meta::parser(|meta| {
		if meta.path.is_ident("sub_dir") {
			sub_dir = Some(meta.value()?.parse()?);
			Ok(())
		} else {
			Err(meta.error("unsupported cache_test option"))
		}
	});
	parse_macro_input!(attr with parser);
	let sub_dir = if let Some(lit) = sub_dir {
		quote! {
			::core::option::Option::Some(::std::borrow::Cow::Borrowed(#lit))
		}
	} else {
		quote! {
			::core::option::Option::<::std::borrow::Cow<str>>::None
		}
	};
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
			let (b, parsers) = crate::Bundle::try_new(&alloc, #sub_dir).unwrap();
			b.bind_plugins(parsers);
			#new_fn_name(&b);
		}
	}
	.into()
}
