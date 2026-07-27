use anyhow::{Context as _, Result};
use arrayvec::ArrayString;
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::Context;
use sha1::{Digest as _, Sha1};
use std::{fmt::Write as _, num::NonZeroU64};
use tracing::error;

use crate::fw::{CommandCtx, CommandFramework};

#[derive(Parser, SlashArgs)]
struct PasswordArgs {
	/// the password to check
	password: String,
}

async fn check_hibp(
	password: &str,
	fw: &CommandFramework,
) -> Result<Option<NonZeroU64>> {
	let sha1 = Sha1::digest(password.as_bytes());
	// sha1 is 20 bytes, so max hex repr is 40 chars
	const MAX_DIGEST_LEN: usize = 20 * 2;
	let mut digest_str: ArrayString<MAX_DIGEST_LEN> = ArrayString::new_const();
	for byte in sha1 {
		write!(digest_str, "{byte:02X}").unwrap();
	}
	let query = &digest_str[..5];
	let suffix = &digest_str[5..];
	let url = format!("https://api.pwnedpasswords.com/range/{query}");
	let res = fw
		.http
		.get(url)
		.send()
		.await
		.context("Failed to send request to HIBP")?
		.error_for_status()
		.context("HIBP returned an error")?;
	for line in res
		.text()
		.await
		.context("Failed to read response from HIBP")?
		.lines()
	{
		if let Some((suff, count)) = line.split_once(':')
			&& suff == suffix
		{
			let ret = count
				.parse::<u64>()
				.context("Failed to parse count from HIBP")?;
			return Ok(Some(
				NonZeroU64::new(ret).context("Count from HIBP was zero")?,
			));
		}
	}

	Ok(None)
}

/// Test the strength of a password with zxcvbn by Dropbox
#[command]
#[arg_parser = PasswordArgs]
#[slash_args]
async fn password(
	args: PasswordArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let strength = zxcvbn::zxcvbn(&args.password, &[]);
	let mut ret = String::from("```yaml\n");
	writeln!(ret, "Guesses: {}", strength.guesses()).unwrap();

	match check_hibp(&args.password, fw).await {
		Ok(Some(count)) => {
			writeln!(ret, "Password found in {count} known breaches")
		}
		Ok(None) => {
			writeln!(ret, "Password not found in known breaches")
		}
		Err(e) => {
			error!("Failed to check HIBP: {:?}", e);
			writeln!(ret, "Failed to check HIBP")
		}
	}
	.unwrap();
	writeln!(ret, "Crack Times:").unwrap();
	let ct = strength.crack_times();
	writeln!(
		ret,
		"- Throttled (100/hr): {}",
		ct.online_throttling_100_per_hour()
	)
	.unwrap();
	writeln!(
		ret,
		"- Fast (10,000/sec): {}",
		ct.offline_slow_hashing_1e4_per_second()
	)
	.unwrap();
	if let Some(f) = strength.feedback() {
		let s = f.suggestions();
		if !s.is_empty() {
			writeln!(ret, "Suggestions:").unwrap();
			for s in s {
				writeln!(ret, "- {s}").unwrap();
			}
		}
		if let Some(w) = f.warning() {
			writeln!(ret, "Warnings:").unwrap();
			writeln!(ret, "- {w}").unwrap();
		}
	}
	while ret.ends_with('\n') {
		ret.pop();
	}
	ret.push_str("\n```");
	cctx.followup_text(ctx, ret)
		.await
		.context("Failed to followup to password cmd")?;
	Ok(())
}
