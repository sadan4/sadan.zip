use std::time::Instant;

use anyhow::{Context as _, Result};
use macros::command;
use serenity::all::{Context, EditMessage, Message};

use crate::{ShardInfo, util::MESSAGE_RECEIVE_TIME};

#[command]
async fn ping(ctx: &Context, msg: &Message) -> Result<()> {
	use std::fmt::Write as _;
	let handler_start = Instant::now();
	let thinking_duration = handler_start - MESSAGE_RECEIVE_TIME.get();
	let gateway_latency = {
		let lock1 = ctx.data.read().await;
		let shard_map = lock1
			.get::<ShardInfo>()
			.context("ShardInfo not found")?;
		let lock2 = shard_map.0.lock().await;
		lock2[&ctx.shard_id].latency
	};
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
	let mut send_msg = msg.reply_ping(ctx, r.clone()).await?;
	let send_duration = send_start.elapsed();
	r.truncate(after_truncate_len);
	writeln!(r, "API latency: {send_duration:.2?}")?;
	send_msg.edit(ctx, EditMessage::new().content(r)).await.context("Failed to edit message")?;
	Ok(())
}
