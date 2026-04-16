use std::{env, process};

use anyhow::{Context, Result, bail};
use tracing::debug;

use crate::util::cmd::{CommandExt, resolve_program_in_path};

pub fn pnpm_i() -> Result<()> {
	if env::var("CI").is_ok() {
		bail!("auto pnpm install is not supported in CI");
	}
	let pnpm_path = resolve_program_in_path("pnpm").context("Failed to find pnpm in PATH")?;
	debug!("Found pnpm at {pnpm_path:?}");
	process::Command::new(pnpm_path)
		.arg("install")
		.run()
		.with_context(|| "Failed to install run pnpm install")
}
