use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::Context;

use crate::fw::CommandCtx;

mod demangler;
mod password;
mod ping;
mod qalc;
mod wolfram;
pub mod wp;
mod userinfo;
mod chmod;
mod loss;
mod hammer;
mod ffmpreg;
mod randomvnc;
mod anti;
mod select;

#[command]
#[sub_cmds[
	ping::ping,
	dev::dev,
	obliterate,
	wp::webpack,
	qalc::qalc,
	version,
	demangler::demangle,
	password::password,
	wolfram::wolfram,
	userinfo::user_info,
	chmod::chmod,
	loss::loss,
	hammer::hammer,
	ffmpreg::ffmpreg,
	anti::anti,
	select::select,
	select::show,
]]
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

#[command]
async fn version(ctx: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	let ver_str = format!("Commit: {}\nBuilt with rust!", git_hash::GIT_HASH);
	cctx.reply(ctx, ver_str)
		.await
		.context("Failed to respond to interaction")?;
	Ok(())
}
