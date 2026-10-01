use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
	Expr,
	LitStr,
	Pat,
	Token,
	parse::{Parse, ParseStream},
	punctuated::Punctuated,
};

struct MatchOverload {
	r#let: Token![let],
	cond: Pat,
	eq: Token![=],
	expr: Expr,
	msg: Message,
}

struct ExprOverload {
	expr: Expr,
	msg: Message,
}

impl Parse for ExprOverload {
	fn parse(i: ParseStream) -> syn::Result<Self> {
		let expr = i.parse()?;
		let _: Option<Token![,]> = i
			.peek(Token![,])
			.then(|| i.parse())
			.transpose()?;
		let msg = if i.is_empty() {
			Message::default_expr(&expr)
		} else {
			i.parse()?
		};
		Ok(Self { expr, msg })
	}
}

fn is_pat_match(s: TokenStream) -> bool {
	if let Some(TokenTree::Ident(i)) = s.into_iter().next()
		&& i == "let"
	{
		true
	} else {
		false
	}
}

impl ExprOverload {
	fn compile(self, is_never: bool) -> TokenStream {
		let inv = if is_never {
			quote! {}
		} else {
			quote! {!}
		};
		let Self { expr, msg } = self;
		let toks = quote! {
			{
				let __cond: ::bool = #expr;
				if #inv __cond {
					if cfg!(debug_assertions) {
						::core::panic!(#msg);
					} else {
						::tracing::error!(#msg)
					}
				}
				__cond
			}
		};
		toks
	}
}

impl MatchOverload {
	fn compile(self, is_never: bool) -> TokenStream {
		let inv = if is_never {
			quote! {}
		} else {
			quote! {!}
		};
		let Self {
			r#let,
			cond,
			eq,
			expr,
			msg,
		} = self;
		let toks = quote! {
			#r#let #cond #eq {
				let __cond = #expr;
				if #inv __cond {
					if cfg!(debug_assertions) {
						::core::panic!(#msg);
					} else {
						::tracing::error!(#msg)
					}
				}
				__cond
			}
		};
		toks
	}
}

impl Parse for MatchOverload {
	fn parse(i: ParseStream) -> syn::Result<Self> {
		let r#let = i.parse()?;
		let cond = Pat::parse_multi_with_leading_vert(i)?;
		let eq = i.parse()?;
		let expr = i.parse()?;
		let _comma: Option<Token![,]> = i
			.peek(Token![,])
			.then(|| i.parse())
			.transpose()?;
		let msg = if i.is_empty() {
			Message::default_match(r#let, &cond, eq, &expr)
		} else {
			i.parse()?
		};
		Ok(Self {
			r#let,
			cond,
			eq,
			expr,
			msg,
		})
	}
}

struct Message {
	fmt: LitStr,
	args: Punctuated<Expr, Token![,]>,
}

impl Message {
	fn default_expr(expr: &Expr) -> Self {
		let toks = quote! {
			"Assertion failed: {}", ::core::stringify!(#expr)
		};
		syn::parse2(toks).unwrap()
	}
	fn default_match(
		r#let: Token![let],
		cond: &Pat,
		eq: Token![=],
		expr: &Expr,
	) -> Self {
		let toks = quote! {
			"Assertion failed: {}", ::core::stringify!(#r#let #cond #eq #expr)
		};
		syn::parse2(toks).unwrap()
	}
}

impl Parse for Message {
	fn parse(i: ParseStream) -> syn::Result<Self> {
		let fmt = i.parse()?;
		let args = Punctuated::parse_terminated(i)?;
		Ok(Self { fmt, args })
	}
}

impl ToTokens for Message {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		self.fmt.to_tokens(tokens);
		if !self.args.is_empty() {
			<Token![,]>::default().to_tokens(tokens);
			self.args.to_tokens(tokens);
		}
	}
}

pub fn always(toks: TokenStream) -> syn::Result<TokenStream> {
	if is_pat_match(toks.clone()) {
		Ok(syn::parse2::<MatchOverload>(toks)?.compile(false))
	} else {
		Ok(syn::parse2::<ExprOverload>(toks)?.compile(false))
	}
}

pub fn never(toks: TokenStream) -> syn::Result<TokenStream> {
	if is_pat_match(toks.clone()) {
		Ok(syn::parse2::<MatchOverload>(toks)?.compile(true))
	} else {
		Ok(syn::parse2::<ExprOverload>(toks)?.compile(true))
	}
}
