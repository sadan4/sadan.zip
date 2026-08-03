use anyhow::{Result, bail};
use macros::command;
use serenity::all::Context;

use crate::fw::CommandCtx;

/// Convert media files
#[command]
async fn ffmpreg(_ctx: &Context, _cctx: &CommandCtx<'_>) -> Result<()> {
	bail!("TODO - Implement");
}
