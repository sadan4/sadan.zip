mod board;
mod paige;
mod prof_heap;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::{Context, Mentionable};

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{self, FROM_REPLY, REFERENCED_USER, UserArg},
};

/// Developer and debugging commands.
#[command]
#[group]
#[sub_cmds(
	ref_user,
	panic,
	error,
	register,
	show_config,
	board::board,
	paige::paige,
	prof_heap::prof_heap,
	stop,
	exit,
	abort,
	segfault,
	trim,
	ping_user
)]
struct Dev;

/// Register all commands globally (overwrites the global command set;
/// propagation can take up to an hour).
#[command]
#[checks(crate::fw::OWNER)]
async fn register(
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	fw.register_global_commands(ctx).await?;
	cctx.reply(ctx, "Registered all commands globally.")
		.await?;
	Ok(())
}

/// Show the loaded bot config (token redacted).
#[command]
#[checks(crate::fw::OWNER)]
async fn show_config(
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	let dbg_repr = format!("{:#?}", fw.config);
	let guh = util::wrap_code_block(&dbg_repr, "ron");
	cctx.reply(ctx, guh).await?;
	Ok(())
}

#[derive(Parser, SlashArgs)]
struct RefUserParser {
	/// The user to reference (mention or id); defaults to the replied-to user.
	#[arg(default_value = FROM_REPLY)]
	user: UserArg,
}

/// Echo the resolved referenced user.
#[command]
#[arg_parser = RefUserParser]
#[slash_args]
async fn ref_user(
	args: RefUserParser,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	use std::fmt::Write as _;
	let mut r = String::new();
	writeln!(r, "referenced user: {}", args.user)?;
	writeln!(r, "REFERENCED_USER CONTEXT: {:?}", REFERENCED_USER.get())?;
	cctx.reply(ctx, r).await?;
	Ok(())
}

/// Intentionally panic, to exercise panic handling.
#[command]
async fn panic(_: &Context, _: &CommandCtx<'_>) -> Result<()> {
	panic!("intentional command panic");
}

#[derive(Parser)]
struct ErrorParser {
	/// Optional message to include in the error.
	#[arg()]
	msg: Option<String>,
}

/// Intentionally return an error, to exercise error handling.
#[command]
#[arg_parser = ErrorParser]
async fn error(
	args: ErrorParser,
	_: &Context,
	_: &CommandCtx<'_>,
) -> Result<()> {
	if let Some(msg) = args.msg {
		bail!("intentional command error: {msg}");
	}
	bail!("intentional command error");
}

#[command]
#[checks(crate::fw::OWNER)]
/// Run [`libc::malloc_trim`]
async fn trim(c: &Context, cc: &CommandCtx<'_>) -> Result<()> {
	cc.defer(c).await?;
	tokio::task::spawn_blocking(|| {
		// SAFETY: safe
		unsafe { libc::malloc_trim(0) };
	})
	.await?;
	cc.followup_text(c, "Trimmed Heap")
		.await?;
	Ok(())
}

#[command]
#[checks(crate::fw::OWNER)]
/// Gracefully stop the bot process.
async fn stop(c: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	_ = cctx.reply(&c, "Exiting...").await;
	c.shutdown_all();
	Ok(())
}

#[command]
#[checks(crate::fw::OWNER)]
/// exit the bot process
async fn exit(c: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	_ = cctx.reply(&c, "Killing...").await;
	std::process::exit(1);
}

#[command]
#[checks(crate::fw::OWNER)]
/// abort the bot process
async fn abort(c: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	_ = cctx.reply(&c, "Terminating...").await;
	std::process::abort();
}

#[command]
#[checks(crate::fw::OWNER)]
/// abort the bot process (via segfault)
async fn segfault(c: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	_ = cctx.reply(&c, "Segfaulting...").await;
	// SAFETY: this is a segfault lol
	unsafe {
		let p: *mut i32 = std::ptr::null_mut();
		*p = 0;
	};
	Ok(())
}

#[derive(Parser, SlashArgs)]
struct PingUserArgs {
	#[arg(default_value = FROM_REPLY)]
	user: UserArg,
}

#[command]
#[checks(crate::fw::OWNER)]
#[arg_parser = PingUserArgs]
#[slash_args]
async fn ping_user(
	user: PingUserArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	cctx.reply(ctx, format!("{}", user.user.mention()))
		.await
		.context("Failed to send ping")?;
	Ok(())
}
