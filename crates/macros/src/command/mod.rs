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

fn se(span: &impl Spanned, msg: impl Display) -> syn::Error {
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
						&span,
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

/// Parse `#[prefix_only]` / `#[slash_only]` into an `Availability` expression.
/// Absent both, a command is available through every front-end.
fn availability(attrs: &mut Vec<Attribute>) -> syn::Result<TokenStream> {
	let prefix_only = take_flag_attr(attrs, "prefix_only");
	let slash_only = take_flag_attr(attrs, "slash_only");
	if let (Some(_), Some(slash)) = (&prefix_only, &slash_only) {
		return Err(se(
			slash,
			"a command cannot be both `#[prefix_only]` and `#[slash_only]`",
		));
	}
	Ok(if prefix_only.is_some() {
		quote! { crate::fw::Availability::PREFIX }
	} else if slash_only.is_some() {
		quote! { crate::fw::Availability::SLASH }
	} else {
		quote! { crate::fw::Availability::all() }
	})
}

/// Parse the `#[slash_args]` flag into the command's `slash_schema` field. When
/// set, the command's `arg_parser` must derive `SlashArgs`, and its native
/// option kinds are used at registration.
fn slash_schema(
	attrs: &mut Vec<Attribute>,
	parser: Option<&Path>,
) -> syn::Result<TokenStream> {
	let Some(flag) = take_flag_attr(attrs, "slash_args") else {
		return Ok(quote! { ::std::option::Option::None });
	};
	let Some(parser) = parser else {
		return Err(se(
			&flag,
			"`#[slash_args]` requires `#[arg_parser = ...]`",
		));
	};
	Ok(quote! {
		::std::option::Option::Some(
			<#parser as crate::fw::SlashSchema>::slash_option_kinds
				as crate::fw::SlashSchemaFn
		)
	})
}

/// Collect `///` doc comments into the command's description. Discord requires
/// slash commands to carry a description; when none is written, `None` is
/// emitted and registration falls back to the command name.
fn command_desc(attrs: &[Attribute]) -> TokenStream {
	let mut lines: Vec<String> = Vec::new();
	for attr in attrs {
		if let Meta::NameValue(kv) = &attr.meta
			&& kv.path.is_ident("doc")
			&& let Expr::Lit(ExprLit {
				lit: Lit::Str(s), ..
			}) = &kv.value
		{
			lines.push(s.value().trim().to_owned());
		}
	}
	let joined = lines.join(" ").trim().to_owned();
	if joined.is_empty() {
		quote! { ::std::option::Option::None }
	} else {
		quote! { ::std::option::Option::Some(#joined) }
	}
}

pub fn arg_parser(attrs: &mut Vec<Attribute>) -> syn::Result<Option<Path>> {
	let Some(kv) = take_kv_attr(attrs, "arg_parser") else {
		return Ok(None);
	};
	match kv.meta {
		Meta::Path(_) | Meta::List(_) => unreachable!(),
		Meta::NameValue(kv) => match kv.value {
			Expr::Path(PatPath { path, .. }) => Ok(Some(path)),
			span => Err(se(&span, "arg_parser must be a path")),
		},
	}
}

pub fn init_attr(attrs: &mut Vec<Attribute>) -> syn::Result<Option<Path>> {
	let Some(kv) = take_kv_attr(attrs, "init") else {
		return Ok(None);
	};
	match kv.meta {
		Meta::Path(_) | Meta::List(_) => unreachable!(),
		Meta::NameValue(kv) => match kv.value {
			Expr::Path(PatPath { path, .. }) => Ok(Some(path)),
			span => Err(se(&span, "init must be a path")),
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
				&p,
				"Sub-command path must have at least one segment",
			));
		};
		match seg.arguments {
			PathArguments::None => {}
			_ => {
				return Err(se(
					&seg,
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
						Output = ::anyhow::Result<
							::std::boxed::Box<
								dyn crate::fw::CommandExecutor + ::std::marker::Send + ::std::marker::Sync + 'static
							>,
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
						::std::result::Result::Ok(
							::std::boxed::Box::new(unsafe {
								::std::mem::MaybeUninit::<#name>::uninit().assume_init()
							})
								as ::std::boxed::Box<
									dyn crate::fw::CommandExecutor + ::std::marker::Send + ::std::marker::Sync + 'static
								>,
						),
					),
				)
			}
			__opaque_executor_factory_zst
		})
	}
}

fn make_init_factory(init: &Path) -> TokenStream {
	quote! {
		crate::fw::OpaqueExecutor::from_const(|__fw| {
			::std::boxed::Box::pin(async move {
				::std::result::Result::Ok(
					::std::boxed::Box::new(#init(__fw).await?)
						as ::std::boxed::Box<
							dyn crate::fw::CommandExecutor
								+ ::std::marker::Send + ::std::marker::Sync + 'static
						>
				)
			})
		})
	}
}

fn executor_factory(
	item: &ItemStruct,
	is_group: bool,
	init: Option<&Path>,
) -> syn::Result<TokenStream> {
	if let Some(init) = init {
		Ok(make_init_factory(init))
	} else if is_group {
		Ok(quote! {
			crate::fw::OpaqueExecutor::__todo()
		})
	} else if item.fields.is_empty() {
		Ok(make_zero_context_factory(&item.ident))
	} else {
		Err(se(
			&item.fields,
			"stateful command (struct with fields) requires `#[init = path]` \
			 to construct its session context",
		))
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

fn make_parser(parser: Option<&Path>, _cmd_ident: &Ident) -> TokenStream {
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
	_attr: &TokenStream,
	mut st: ItemStruct,
) -> syn::Result<TokenStream> {
	let name = command_name(&mut st.attrs, &st.ident)?;
	let checks = checks(&mut st.attrs)?;
	let arg_parser = arg_parser(&mut st.attrs)?;
	let init = init_attr(&mut st.attrs)?;
	let sub_cmds = sub_cmds(&mut st.attrs)?;
	let is_group = group(&mut st.attrs);
	let is_root_group =
		is_group && take_flag_attr(&mut st.attrs, "root").is_some();
	let early_init = take_flag_attr(&mut st.attrs, "early_init");
	if let Some(attr) = &early_init
		&& init.is_none()
	{
		return Err(se(
			attr,
			"`#[early_init]` requires `#[init = path]`: only stateful \
				 commands have session context to pre-load",
		));
	}
	let availability = availability(&mut st.attrs)?;
	let slash_schema = slash_schema(&mut st.attrs, arg_parser.as_ref())?;
	let desc = command_desc(&st.attrs);
	let CommandNames {
		cmd_ident,
		context_type_ident,
		..
	} = CommandNames::new(&name);
	let factory = executor_factory(&st, is_group, init.as_ref())?;
	let context_struct_ident = &st.ident;
	let root_group_flag = is_root_group
		.then(|| quote! {.union(crate::fw::CommandFlags::ROOT_GROUP)});
	let group_flag =
		is_group.then(|| quote! {.union(crate::fw::CommandFlags::GROUP)});
	let early_init_flag = early_init
		.is_some()
		.then(|| quote! {.union(crate::fw::CommandFlags::EAGER_INIT)});
	let parser = make_parser(arg_parser.as_ref(), &cmd_ident);
	// a pure group is a marker type that is never instantiated
	let group_dead_code = is_group.then(|| quote! { #[allow(dead_code)] });
	let toks = quote! {
		#group_dead_code
		#st

		pub static #cmd_ident: crate::fw::Command = crate::fw::Command {
			checks: &[#(#checks),*],
			names: &[#name],
			parser: #parser,
			desc: #desc,
			usage_location: crate::fw::UsageLocation::all(),
			sub_cmds: &[#(& #sub_cmds),*],
			executor: #factory,
			flags: crate::fw::CommandFlags::NONE #group_flag #root_group_flag #early_init_flag,
			availability: #availability,
			slash_schema: #slash_schema,
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
) -> TokenStream {
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
				__cctx: &crate::fw::CommandCtx<'_>,
				__cmd: &crate::fw::Command,
				__fw: &crate::fw::CommandFramework,
				__args: &::clap::ArgMatches,
			) -> ::anyhow::Result<()> {
				let __parsed_args = #parse_args;
				let __result: ::anyhow::Result<()> = #func_ident(#pass_args __ctx, __cctx, #pass_extra_args).await;
				__result
			}
		}
	};
	res
}

fn command_func(
	_attr: &TokenStream,
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
	let is_group = group(&mut func.attrs);
	let availability = availability(&mut func.attrs)?;
	let slash_schema = slash_schema(&mut func.attrs, parser.as_ref())?;
	let desc = command_desc(&func.attrs);
	let group_flag =
		is_group.then(|| quote! {.union(crate::fw::CommandFlags::GROUP)});
	let executor_impl = make_executor_impl_for_command_func(
		&struct_ident,
		&func.sig.ident,
		parser.as_ref(),
		num_extra_args,
	);
	let factory = make_zero_context_factory(&struct_ident);
	let parser = make_parser(parser.as_ref(), &cmd_ident);
	let res = quote! {
		// the handler is required to be async (it is awaited during dispatch),
		// even if a given command never awaits
		#[allow(clippy::unused_async)]
		#func
		pub struct #struct_ident;
		pub type #context_type_ident = #struct_ident;
		#executor_impl
		pub static #cmd_ident: crate::fw::Command = crate::fw::Command {
			checks: &[#(#checks),*],
			names: &[#name],
			parser: #parser,
			desc: #desc,
			usage_location: crate::fw::UsageLocation::all(),
			sub_cmds: &[#(& #sub_cmds),*],
			executor: #factory,
			flags: crate::fw::CommandFlags::NONE #group_flag,
			availability: #availability,
			slash_schema: #slash_schema,
		};
	};
	Ok(res)
}

/// If `ty` is `Option<T>`, returns `T`; otherwise returns `ty` unchanged. Used
/// so an optional argument registers with the same native option kind as its
/// required counterpart.
fn unwrap_option(ty: &syn::Type) -> &syn::Type {
	if let syn::Type::Path(tp) = ty
		&& tp.qself.is_none()
		&& let Some(seg) = tp.path.segments.last()
		&& seg.ident == "Option"
		&& let PathArguments::AngleBracketed(args) = &seg.arguments
		&& let Some(syn::GenericArgument::Type(inner)) = args.args.first()
	{
		return inner;
	}
	ty
}

/// Derive `SlashSchema`: emit each field's `(name, discord_kind)` by reading
/// the field type through the `SlashArg` trait.
pub fn slash_args_derive(item: TokenStream) -> syn::Result<TokenStream> {
	let input: ItemStruct = parse2(item)?;
	let ident = &input.ident;
	let mut entries = Vec::new();
	for field in &input.fields {
		let Some(fname) = &field.ident else {
			return Err(se(
				&field.ty,
				"SlashArgs can only be derived for structs with named fields",
			));
		};
		let name = fname.to_string();
		let ty = unwrap_option(&field.ty);
		entries.push(quote! {
			(#name, <#ty as crate::fw::SlashArg>::KIND)
		});
	}
	Ok(quote! {
		impl crate::fw::SlashSchema for #ident {
			fn slash_option_kinds() -> ::std::vec::Vec<(
				&'static str,
				::serenity::all::CommandOptionType,
			)> {
				::std::vec![ #(#entries),* ]
			}
		}
	})
}

pub fn command(
	attr: &TokenStream,
	item: TokenStream,
) -> syn::Result<TokenStream> {
	let input = parse2(item)?;
	match input {
		CommandTarget::Function(func) => command_func(attr, func),
		CommandTarget::Struct(st) => command_struct(attr, st),
	}
}

fn verify_command_func_sig(sig: &Signature) -> syn::Result<()> {
	if let Some(kw_const) = &sig.constness {
		return Err(se(kw_const, "executor function must not be const"));
	}
	if sig.asyncness.is_none() {
		return Err(se(&sig.ident, "executor function must be async"));
	}
	if let Some(kw_unsafe) = &sig.unsafety {
		return Err(se(kw_unsafe, "executor function must not be unsafe"));
	}
	if let Some(abi) = &sig.abi {
		return Err(se(abi, "executor function must not have an explicit ABI"));
	}
	if !sig.generics.params.is_empty() {
		return Err(se(
			&sig.generics.params.span(),
			"executor function must not have generic parameters",
		));
	}
	if let Some(varargs) = &sig.variadic {
		return Err(se(varargs, "executor function must not be variadic"));
	}
	Ok(())
}

/// Extracts the referenced type of a shared reference `&T`, or `None` for a
/// non-reference type.
fn reference_inner(ty: &syn::Type) -> Option<&syn::Type> {
	match ty {
		syn::Type::Reference(r) => Some(&r.elem),
		_ => None,
	}
}

pub fn executor(
	_attr: TokenStream,
	item: TokenStream,
) -> syn::Result<TokenStream> {
	let input: ItemFn = parse2(item)?;
	verify_command_func_sig(&input.sig)?;
	let fn_ident = &input.sig.ident;

	// collect the typed parameters; a receiver (`self`) is not allowed
	let mut params = Vec::new();
	for arg in &input.sig.inputs {
		let FnArg::Typed(t) = arg else {
			return Err(se(
				arg,
				"executor function must not have a receiver argument",
			));
		};
		params.push(t);
	}

	// a leading parameter passed *by value* is the parsed argument struct; it is
	// deserialized from the `ArgMatches` via `FromArgMatches`. Everything the
	// handler otherwise takes is by shared reference, so this is unambiguous.
	let (arg_parse, rest) = match params.split_first() {
		Some((first, tail)) if reference_inner(&first.ty).is_none() => {
			let arg_ty = &first.ty;
			(
				Some(quote! {
					let __parsed_args =
						<#arg_ty as ::clap::FromArgMatches>::from_arg_matches(
							__args,
						)?;
				}),
				tail,
			)
		}
		_ => (None, params.as_slice()),
	};

	// remaining, in order: `state: &State`, `ctx: &Context`,
	// `cctx: &CommandCtx`, then optionally `cmd: &Command` and
	// `fw: &CommandFramework`
	if !(3..=5).contains(&rest.len()) {
		return Err(se(
			&input.sig.inputs,
			"expected signature: `async fn handler(args: Args, state: &State, \
			 ctx: &Context, cctx: &CommandCtx, cmd: &Command, fw: &CommandFramework)`\
			 \nNOTE: `args` is optional; `cmd` and `fw` are optional trailing \
			 parameters",
		));
	}
	let Some(state_ty) = reference_inner(&rest[0].ty) else {
		return Err(se(
			&rest[0],
			"the state parameter must be a shared reference `&State`",
		));
	};

	let pass_args = arg_parse
		.is_some()
		.then(|| quote! { __parsed_args, });
	let pass_extra = match rest.len() - 3 {
		0 => quote! {},
		1 => quote! { __cmd, },
		2 => quote! { __cmd, __fw, },
		_ => unreachable!("rest length checked to be 3..=5"),
	};

	let res = quote! {
		// the executor is required to be async (it is called in an async
		// context via the trait), even if a given handler never awaits
		#[allow(clippy::unused_async)]
		#input

		#[::serenity::async_trait]
		impl crate::fw::CommandExecutor for #state_ty {
			async fn execute(
				&self,
				__ctx: &::serenity::all::Context,
				__cctx: &crate::fw::CommandCtx<'_>,
				__cmd: &crate::fw::Command,
				__fw: &crate::fw::CommandFramework,
				__args: &::clap::ArgMatches,
			) -> ::anyhow::Result<()> {
				#arg_parse
				#fn_ident(
					#pass_args self, __ctx, __cctx, #pass_extra
				).await
			}
		}
	};
	Ok(res)
}
