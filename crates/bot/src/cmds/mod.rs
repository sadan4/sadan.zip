use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::Context;

use crate::fw::CommandCtx;

mod ping;
mod wp;
mod wolfram;

#[command]
#[sub_cmds(ping::ping, dev::dev, obliterate, wp::webpack)]
#[group]
#[root]
struct Root;

#[derive(Parser, SlashArgs)]
struct ObliterateArgs {
	#[arg(default_value = "freslet")]
	target: String,
}

#[command]
#[arg_parser = ObliterateArgs]
#[slash_args]
async fn obliterate(
	args: ObliterateArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	cctx.reply(ctx, format!("Obliterated a {}", args.target))
		.await
		.context("Failed to respond to interaction")?;
	Ok(())
}

mod dev;
