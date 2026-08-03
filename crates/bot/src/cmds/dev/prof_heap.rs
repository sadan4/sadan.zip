use std::{fmt::Write as _, sync::Arc};

use anyhow::{Context as _, Result};
use macros::command;
use serenity::all::Context;
use typesize::TypeSize;

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{
		FormatBytes,
		rss_bytes,
	},
};

#[command]
async fn prof_heap(
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	let mut msg = String::new();
	msg.push_str("```\n");
	// Cache
	{
		let strong_count = Arc::strong_count(&ctx.cache);
		let weak_count = Arc::weak_count(&ctx.cache);
		let size = FormatBytes(ctx.cache.get_size());
		writeln!(msg, "Cache {size} {strong_count} strong {weak_count} weak")?;
	};
	// Conifg
	{
		let strong_count = Arc::strong_count(&fw.config);
		let weak_count = Arc::weak_count(&fw.config);
		let size = FormatBytes(fw.config.get_size());
		writeln!(msg, "Config {size} {strong_count} strong {weak_count} weak")?;
	};
	// Gif Templates
	{
		let tmpls = fw
			.get_gif_templates()
			.await
			.context("Failed to get GIF templates")?;
		let size = FormatBytes(tmpls.get_size());
		writeln!(msg, "GifTemplates {size}")?;
	};
	// Webpack Context
	if let Some(ctx) = fw.get_wp_ctx() {
		let strong_count = Arc::strong_count(&ctx) - 1;
		let weak_count = Arc::weak_count(&ctx);
		let size = FormatBytes(
			tokio::task::spawn_blocking(move || ctx.get_size()).await?,
		);
		writeln!(
			msg,
			"WebpackContext {size} {strong_count} strong {weak_count} weak",
		)?;
	}
	let rss = rss_bytes().await? as usize;
	writeln!(msg, "RSS: {}", FormatBytes(rss))?;
	msg.push_str("```");
	cctx.reply(ctx, msg).await?;
	Ok(())
}
