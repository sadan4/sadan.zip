use std::process;

use anyhow::{Context, Result};
use tracing::debug;

use crate::util::cmd::{CommandExt, resolve_program_in_path};

pub fn pnpm_i() -> Result<()> {
	let pnpm_path = resolve_program_in_path("pnpm").context("Failed to find pnpm in PATH")?;
	debug!("Found pnpm at {pnpm_path:?}");
	process::Command::new(pnpm_path)
		.arg("install")
		.run()
		.with_context(|| "Failed to install run pnpm install")
}
