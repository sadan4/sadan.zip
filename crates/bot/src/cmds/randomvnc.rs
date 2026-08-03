use anyhow::{Result};
use macros::command;
use serenity::all::Context;

use crate::fw::CommandCtx;

/// Short description of the command.
#[command]
async fn random_vnc(_: &Context, _: &CommandCtx<'_>) -> Result<()> {
	todo!("implement");
}
