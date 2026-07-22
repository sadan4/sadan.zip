use std::fmt::Display;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
	Attribute,
	Expr,
	ExprLit,
	FnArg,
	Ident,
	ItemFn,
	ItemStruct,
	Lit,
	Meta,
	PatPath,
	Path,
	PathArguments,
	Signature,
	Token,
	parse::{Parse, ParseStream, discouraged::Speculative},
	parse_macro_input,
	parse2,
	punctuated::Punctuated,
	spanned::Spanned,
};

fn find_list_attr(attrs: &[Attribute], key: &str) -> Option<usize> {
	for (i, attr) in attrs.iter().enumerate() {
		match &attr.meta {
			Meta::List(l) => {
				if l.path.is_ident(key) {
					return Some(i);
				}
			}
			Meta::Path(_) | Meta::NameValue(_) => {}
		}
	}
	None
}

fn find_kv_attr(attrs: &[Attribute], key: &str) -> Option<usize> {
	for (i, attr) in attrs.iter().enumerate() {
		match &attr.meta {
			Meta::Path(_) | Meta::List(_) => {}
			Meta::NameValue(kv) => {
				if kv.path.is_ident(key) {
					return Some(i);
				}
			}
		}
	}
	None
}

fn find_flag_attr(attrs: &[Attribute], key: &str) -> Option<usize> {
	for (i, attr) in attrs.iter().enumerate() {
		match &attr.meta {
			Meta::Path(path) => {
				if path.is_ident(key) {
					return Some(i);
				}
			}
			Meta::List(_) | Meta::NameValue(_) => {}
		}
	}
	None
}

fn take_flag_attr(attrs: &mut Vec<Attribute>, key: &str) -> Option<Attribute> {
	let i = find_flag_attr(attrs, key)?;
	Some(attrs.remove(i))
}

fn take_kv_attr(attrs: &mut Vec<Attribute>, key: &str) -> Option<Attribute> {
	let i = find_kv_attr(attrs, key)?;
	Some(attrs.remove(i))
}

fn take_list_attr(attrs: &mut Vec<Attribute>, key: &str) -> Option<Attribute> {
	let i = find_list_attr(attrs, key)?;
	Some(attrs.remove(i))
}

fn se(span: impl Spanned, msg: impl Display) -> syn::Error {
	syn::Error::new(span.span(), msg)
}

fn pascal_to_snake(s: &str) -> String {
	let mut out = String::with_capacity(s.len() + s.len() / 2);
	for (i, c) in s.char_indices() {
		if c.is_uppercase() && i != 0 {
			out.push('_');
		}
		out.extend(c.to_lowercase());
	}
	out
}

fn command_name(
	attrs: &mut Vec<Attribute>,
	ident: &Ident,
) -> syn::Result<String> {
	let name = if let Some(attr) = take_kv_attr(attrs, "name") {
		match attr.meta {
			Meta::Path(_) | Meta::List(_) => unreachable!(),
			Meta::NameValue(kv) => match kv.value {
				Expr::Lit(ExprLit {
					lit: Lit::Str(s), ..
				}) => s.value(),
				span => {
					return Err(se(
						span,
						"Command name must be a string literal",
					));
				}
			},
		}
	} else {
		pascal_to_snake(&ident.to_string())
	};
	Ok(name)
}

pub fn checks(attrs: &mut Vec<Attribute>) -> syn::Result<Vec<Expr>> {
	let Some(attr) = take_list_attr(attrs, "checks") else {
		return Ok(Vec::new());
	};
	let res = attr
		.meta
		.require_list()?
		.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
		.into_iter()
		.collect();
	Ok(res)
}

pub fn group(attrs: &mut Vec<Attribute>) -> bool {
	take_flag_attr(attrs, "group").is_some()
}

pub fn arg_parser(attrs: &mut Vec<Attribute>) -> syn::Result<Option<Path>> {
	let Some(kv) = take_kv_attr(attrs, "arg_parser") else {
		return Ok(None);
	};
	match kv.meta {
		Meta::Path(_) | Meta::List(_) => unreachable!(),
		Meta::NameValue(kv) => match kv.value {
			Expr::Path(PatPath { path, .. }) => Ok(Some(path)),
			span => Err(se(span, "arg_parser must be a path")),
		},
	}
}

pub fn sub_cmds(attrs: &mut Vec<Attribute>) -> syn::Result<Vec<Path>> {
	let Some(attr) = take_list_attr(attrs, "sub_cmds") else {
		return Ok(Vec::new());
	};
	let mut res: Vec<Path> = attr
		.meta
		.require_list()?
		.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)?
		.into_iter()
		.collect();

	for p in &mut res {
		let Some(seg) = p.segments.last_mut() else {
			return Err(se(
				p,
				"Sub-command path must have at least one segment",
			));
		};
		match seg.arguments {
			PathArguments::None => {}
			_ => {
				return Err(se(
					seg,
					"Sub-command path must not have any generic arguments",
				));
			}
		}
		let mut ident_str = seg.ident.to_string();
		if ident_str.ends_with("_CMD") {
			continue;
		}
		ident_str = ident_str.to_ascii_uppercase();
		ident_str.push_str("_CMD");
		seg.ident = Ident::new(&ident_str, seg.ident.span());
	}

	Ok(res)
}

pub fn screaming_snake(s: &[&str]) -> String {
	let len = s.iter().map(|s| s.len()).sum::<usize>() + (s.len() - 1);
	let mut ret = String::with_capacity(len);
	for (i, s) in s.iter().enumerate() {
		if i != 0 {
			ret.push('_');
		}
		ret.push_str(&s.to_ascii_uppercase());
	}
	ret
}

#[expect(clippy::struct_field_names)]
struct CommandNames {
	struct_ident: Ident,
	cmd_ident: Ident,
	executor_ident: Ident,
	context_type_ident: Ident,
}

impl CommandNames {
	fn new(name: &str) -> Self {
		Self {
			struct_ident: Ident::new(
				&screaming_snake(&[name, "DATA"]),
				Span::call_site(),
			),
			cmd_ident: Ident::new(
				&screaming_snake(&[name, "CMD"]),
				Span::call_site(),
			),
			executor_ident: Ident::new(
				&screaming_snake(&[name, "EXECUTOR"]),
				Span::call_site(),
			),
			context_type_ident: Ident::new(
				&screaming_snake(&[name, "CONTEXT_TYPE"]),
				Span::call_site(),
			),
		}
	}
}

fn make_zero_context_factory(name: &Ident) -> TokenStream {
	quote! {
		crate::fw::OpaqueExecutor::from_const({
			const _: () = {
				::std::assert!(
					::std::mem::size_of::<#name>() == 0,
					"macros::cmd error, make_zero_context_factory must only be called if the context struct is a ZST"
				);
			};
			fn __opaque_executor_factory_zst<'a>(
				_: &'a crate::fw::CommandFramework
			) -> ::std::pin::Pin<
				::std::boxed::Box<
					dyn ::std::future::Future<
						Output = ::std::boxed::Box<
							dyn crate::fw::CommandExecutor + ::std::marker::Send + ::std::marker::Sync + 'static
						>,
					> + ::std::marker::Send + 'a
				>,
			>
			where
				#name: crate::fw::CommandExecutor,
				#name: ::std::marker::Send,
				#name: ::std::marker::Sync,
				#name: 'static
			{
				// SAFETY: this is safe because #name is a ZST
				::std::boxed::Box::pin(
					::std::future::ready(
						::std::boxed::Box::new(unsafe {
							::std::mem::MaybeUninit::<#name>::uninit().assume_init()
						})
							as ::std::boxed::Box<
								dyn crate::fw::CommandExecutor + ::std::marker::Send + ::std::marker::Sync + 'static
							>,
					),
				)
			}
			__opaque_executor_factory_zst
		})
	}
}

fn executor_factory(item: &ItemStruct, is_group: bool) -> TokenStream {
	if is_group {
		quote! {
			crate::fw::OpaqueExecutor::__todo()
		}
	} else if item.fields.is_empty() {
		make_zero_context_factory(&item.ident)
	} else {
		quote! {
			crate::fw::OpaqueExecutor::__todo()
		}
	}
}

enum CommandTarget {
	Function(ItemFn),
	Struct(ItemStruct),
}

impl Parse for CommandTarget {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let fork = input.fork();
		if let Ok(s) = fork.parse() {
			input.advance_to(&fork);
			return Ok(Self::Struct(s));
		}
		let fork = input.fork();
		if let Ok(f) = fork.parse() {
			input.advance_to(&fork);
			return Ok(Self::Function(f));
		}
		Err(syn::Error::new(
			input.span(),
			"expected a function or a struct",
		))
	}
}

fn make_parser(parser: Option<&Path>, cmd_ident: &Ident) -> TokenStream {
	if let Some(parser) = parser {
		quote! {
			crate::fw::ParserFactory::__make_parser::<#parser>()
		}
	} else {
		quote! {
			crate::fw::ParserFactory::__make_null_parser()
		}
	}
}

fn command_struct(
	attr: TokenStream,
	mut st: ItemStruct,
) -> syn::Result<TokenStream> {
	let name = command_name(&mut st.attrs, &st.ident)?;
	let checks = checks(&mut st.attrs)?;
	let arg_parser = arg_parser(&mut st.attrs)?;
	let sub_cmds = sub_cmds(&mut st.attrs)?;
	let is_group = group(&mut st.attrs);
	let is_root_group =
		is_group && take_flag_attr(&mut st.attrs, "root").is_some();
	let CommandNames {
		cmd_ident,
		context_type_ident,
		..
	} = CommandNames::new(&name);
	let factory = executor_factory(&st, is_group);
	let context_struct_ident = &st.ident;
	let root_group_flag = is_root_group
		.then(|| quote! {.union(crate::fw::CommandFlags::ROOT_GROUP)});
	let group_flag =
		is_group.then(|| quote! {.union(crate::fw::CommandFlags::GROUP)});
	let parser = make_parser(arg_parser.as_ref(), &cmd_ident);
	let toks = quote! {
		#st

		pub static #cmd_ident: crate::fw::Command = crate::fw::Command {
			checks: &[#(#checks),*],
			names: &[#name],
			parser: #parser,
			desc: ::std::option::Option::Some("TODO: handle descriptions in macro"),
			usage_location: crate::fw::UsageLocation::all(),
			sub_cmds: &[#(& #sub_cmds),*],
			executor: #factory,
			flags: crate::fw::CommandFlags::NONE #group_flag #root_group_flag,
		};

		pub type #context_type_ident = #context_struct_ident;
	};

	Ok(toks)
}

fn make_executor_impl_for_command_func(
	struct_name: &Ident,
	func_ident: &Ident,
	arg_parser: Option<&Path>,
	extra_args: u8,
) -> syn::Result<TokenStream> {
	let [parse_args, pass_args]: [TokenStream; 2] =
		if let Some(parser) = arg_parser {
			[
				quote! {
					<#parser as ::clap::FromArgMatches>::from_arg_matches(__args)?
				},
				quote! { __parsed_args, },
			]
		} else {
			[quote! {()}, quote! {}]
		};
	let pass_extra_args = match extra_args {
		0 => quote! {},
		1 => quote! { __cmd, },
		2 => quote! { __cmd, __fw, },
		_ => unreachable!("extra_args must be 0, 1, or 2"),
	};
	let res = quote! {
		#[::serenity::async_trait]
		impl crate::fw::CommandExecutor for #struct_name {
			async fn execute(
				&self,
				__ctx: &::serenity::all::Context,
				__msg: &::serenity::all::Message,
				__cmd: &crate::fw::Command,
				__fw: &crate::fw::CommandFramework,
				__args: &::clap::ArgMatches,
			) -> ::anyhow::Result<()> {
				let __parsed_args = #parse_args;
				let __result: ::anyhow::Result<()> = #func_ident(#pass_args __ctx, __msg, #pass_extra_args).await;
				__result
			}
		}
	};
	Ok(res)
}

fn command_func(
	attr: TokenStream,
	mut func: ItemFn,
) -> syn::Result<TokenStream> {
	verify_command_func_sig(&func.sig)?;
	let name = command_name(&mut func.attrs, &func.sig.ident)?;
	let checks = checks(&mut func.attrs)?;
	let parser = arg_parser(&mut func.attrs)?;
	let min_args = 2 + u8::from(parser.is_some());
	let max_args = 4 + u8::from(parser.is_some());
	if !(min_args..=max_args).contains(&(func.sig.inputs.len() as u8)) {
		let err = if parser.is_some() {
			se(
				&func.sig.inputs,
				"expected signature: `async fn cmd_name(args: &CommandArguments, ctx: &Context, msg: &Message, cmd: &Command, fw: &CommandFramework\
		    \nNOTE: the last two parameters are optional and can be omitted",
			)
		} else {
			se(
				&func.sig.inputs,
				"expected signature: `async fn cmd_name(ctx: &Context, msg: &Message, cmd: &Command, fw: &CommandFramework\
		    \nNOTE: the last two parameters are optional and can be omitted",
			)
		};
		return Err(err);
	}
	let num_extra_args = func.sig.inputs.len() as u8 - min_args;
	let CommandNames {
		cmd_ident,
		struct_ident,
		context_type_ident,
		..
	} = CommandNames::new(&name);
	let sub_cmds = sub_cmds(&mut func.attrs)?;
	let executor_impl = make_executor_impl_for_command_func(
		&struct_ident,
		&func.sig.ident,
		parser.as_ref(),
		num_extra_args,
	)?;
	let factory = make_zero_context_factory(&struct_ident);
	let parser = make_parser(parser.as_ref(), &cmd_ident);
	let res = quote! {
		#func
		pub struct #struct_ident;
		pub type #context_type_ident = #struct_ident;
		#executor_impl
		pub static #cmd_ident: crate::fw::Command = crate::fw::Command {
			checks: &[#(#checks),*],
			names: &[#name],
			parser: #parser,
			desc: ::std::option::Option::Some("TODO: handle descriptions in macro"),
			usage_location: crate::fw::UsageLocation::all(),
			sub_cmds: &[#(& #sub_cmds),*],
			executor: #factory,
			flags: crate::fw::CommandFlags::NONE,
		};
	};
	Ok(res)
}

pub fn command(
	attr: TokenStream,
	item: TokenStream,
) -> syn::Result<TokenStream> {
	let input = parse2(item)?;
	match input {
		CommandTarget::Function(func) => command_func(attr, func),
		CommandTarget::Struct(st) => command_struct(attr, st),
	}
}

fn verify_command_func_sig(sig: &Signature) -> syn::Result<()> {
	if let Some(kw_const) = sig.constness {
		return Err(se(kw_const, "executor function must not be const"));
	}
	if sig.asyncness.is_none() {
		return Err(se(&sig.ident, "executor function must be async"));
	}
	if let Some(kw_unsafe) = sig.unsafety {
		return Err(se(kw_unsafe, "executor function must not be unsafe"));
	}
	if let Some(abi) = &sig.abi {
		return Err(se(abi, "executor function must not have an explicit ABI"));
	}
	if !sig.generics.params.is_empty() {
		return Err(se(
			sig.generics.params.span(),
			"executor function must not have generic parameters",
		));
	}
	if let Some(varargs) = &sig.variadic {
		return Err(se(varargs, "executor function must not be variadic"));
	}
	Ok(())
}

fn verify_executor_signature(sig: &Signature) -> syn::Result<()> {
	verify_command_func_sig(sig)?;
	if sig.inputs.len() != 6 {
		return Err(se(
			sig.inputs.span(),
			"executor function must have exactly 6 parameters.\
		\nExpected signature: \
		`async fn cmd_name(this: &CommandData, ctx: &Context, msg: &Message, cmd: &'static Command, fw: &'static CommandFramework, args: &ArgMatches) -> ()`",
		));
	}
	for arg in &sig.inputs {
		let FnArg::Typed(arg) = arg else {
			return Err(se(
				arg,
				"executor function must not have a receiver argument",
			));
		};
	}
	Ok(())
}

pub fn executor(
	_attr: TokenStream,
	item: TokenStream,
) -> syn::Result<TokenStream> {
	let mut input: ItemFn = parse2(item)?;
	verify_executor_signature(&input.sig)?;
	let fn_ident = &input.sig.ident;
	let name = command_name(&mut input.attrs, fn_ident)?;
	let CommandNames {
		context_type_ident, ..
	} = CommandNames::new(&name);
	let res = quote! {
		#input

		#[::serenity::async_trait]
		impl crate::fw::CommandExecutor for #context_type_ident {
			async fn execute(
				&self,
				ctx: &::serenity::all::Context,
				msg: &::serenity::all::Message,
				cmd: &'static crate::fw::Command,
				fw: &'static crate::fw::CommandFramework,
				args: &::clap::ArgMatches
			) -> () {
				#fn_ident(self, ctx, msg, cmd, fw, args).await
			}
		}
	};
	Ok(res)
}
