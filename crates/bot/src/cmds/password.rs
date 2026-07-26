use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::Context;

use crate::fw::CommandCtx;

#[derive(Parser, SlashArgs)]
struct PasswordArgs {
	/// the password to check
	password: String,
}

/// Test the strength of a password with zxcvbn by Dropbox
#[command]
#[arg_parser = PasswordArgs]
#[slash_args]
async fn password(
	args: PasswordArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let strength = zxcvbn::zxcvbn(&args.password, &[]);
	let mut ret = String::from("```yaml\n");
	ret.push_str(
		&yaml_serde::to_string(&strength)
			.context("Failed to serialize password strength")?,
	);
	ret.push_str("\n```");
	cctx.followup_text(ctx, ret)
		.await
		.context("Failed to followup to password cmd")?;
	Ok(())
}
