use crate::fw::{CommandCtx, SlashOption, SlashSchema};
use anyhow::{Context as _, Result};
use clap::Parser;
use itertools::Itertools as _;
use macros::command;
use serenity::all::{CommandOptionType, Context};

#[derive(Parser)]
struct DemanglerArgs {
	/// The symbols to demangle. Anything can be provided, only symbols will be demangled.
	#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
	expr: Vec<String>,
}

impl SlashSchema for DemanglerArgs {
	fn slash_options() -> Vec<crate::fw::SlashOption> {
		vec![SlashOption {
			choices: vec![],
			kind: CommandOptionType::String,
			name: "expr",
		}]
	}
}

#[command]
#[arg_parser = DemanglerArgs]
#[slash_args]
async fn demangle(
	args: DemanglerArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer command")?;
	let mut res = args
		.expr
		.into_iter()
		.map(|s| {
			let demangled = demangler::ffi::demangle_all(&s);
			if demangled.is_null() {
				s
			} else {
				demangled
					.to_str()
					.expect("demangled output is not utf8")
					.to_string()
			}
		})
		.join(" ");
	// ```\n{...}\n```
	const MAX_LEN: usize = 2000 - 8;
	while res.ends_with('\n') {
		res.pop();
	}
	while res.starts_with('\n') {
		res.remove(0);
	}
	if res.len() > MAX_LEN {
		cctx.followup_text(ctx, "Output too long, truncating...")
			.await?;
	}
	res.truncate(MAX_LEN);

	res.insert_str(0, "```\n");
	res.push_str("\n```");
	cctx.followup_text(ctx, res).await?;
	Ok(())
}
