use clap::Parser;
use anyhow::{Context as _, Result};
use macros::command;
use serenity::all::{Context, Message};

#[derive(Parser)]
struct PingArgs {
	#[arg(long)]
	msg: String,
}

#[command]
#[arg_parser = PingArgs]
async fn ping(args: PingArgs, ctx: &Context, msg: &Message) -> Result<()> {
	msg.reply_ping(&ctx.http, args.msg)
		.await
		.context("Failed to reply")?;
	Ok(())
}
