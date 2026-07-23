use anyhow::{Result, bail};
use clap::Parser;
use macros::command;
use serenity::all::{Context, Message};

use crate::util::{FROM_REPLY, REFERENCED_USER, UserArg};

#[command]
#[group]
#[sub_cmds(ref_user, panic, error)]
struct Dev;

#[derive(Parser)]
struct RefUserParser {
	#[arg(default_value = FROM_REPLY)]
	user: UserArg,
}

#[command]
#[arg_parser = RefUserParser]
async fn ref_user(
	args: RefUserParser,
	ctx: &Context,
	msg: &Message,
) -> Result<()> {
	use std::fmt::Write as _;
	let mut r = String::new();
	writeln!(r, "referenced user: {}", args.user)?;
	writeln!(r, "REFERENCED_USER CONTEXT: {:?}", REFERENCED_USER.get())?;
	msg.reply_ping(ctx, r).await?;
	Ok(())
}

#[command]
async fn panic(_: &Context, _: &Message) -> Result<()> {
	panic!("intentional command panic");
}

#[derive(Parser)]
struct ErrorParser {
	#[arg()]
	msg: Option<String>,
}

#[command]
#[arg_parser = ErrorParser]
async fn error(args: ErrorParser, _: &Context, _: &Message) -> Result<()> {
	if let Some(msg) = args.msg {
		bail!("intentional command error: {msg}");
	}
	bail!("intentional command error");
}
