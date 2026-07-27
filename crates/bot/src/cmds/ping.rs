use std::time::Instant;

use anyhow::{Context as _, Result};
use macros::command;
use serenity::all::Context;

use crate::{fw::CommandCtx, util::MESSAGE_RECEIVE_TIME};

/// Check the bot's gateway and API latency.
#[command]
async fn ping(ctx: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	use std::fmt::Write as _;
	let handler_start = Instant::now();
	let thinking_duration = handler_start - MESSAGE_RECEIVE_TIME.get();
	let gateway_latency = ctx.runner_info.read().latency;
	let mut r = String::new();
	writeln!(r, "Pong 🏓")?;
	writeln!(r, "Thinking time: {thinking_duration:.2?}")?;
	if let Some(latency) = gateway_latency {
		writeln!(r, "Gateway latency: {latency:.2?}")?;
	} else {
		writeln!(r, "Gateway latency: unknown")?;
	}
	let after_truncate_len = r.len();
	writeln!(r, "API latency: ...")?;
	let send_start = Instant::now();
	let mut reply = cctx.reply(ctx, r.clone()).await?;
	let send_duration = send_start.elapsed();
	r.truncate(after_truncate_len);
	writeln!(r, "API latency: {send_duration:.2?}")?;
	reply
		.edit(ctx, r)
		.await
		.context("Failed to edit message")?;
	Ok(())
}
