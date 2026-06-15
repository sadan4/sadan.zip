use std::process;

use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	build,
	util::cmd::{CommandExt as _, resolve_program_in_path},
};

#[derive(Args, Debug)]
pub struct Command {
	/// Package the extension in development mode (skip minification, use a
	/// debug LSP binary). Passed through to the build step.
	#[arg(long, default_value_t = false)]
	pub dev: bool,
}

impl Command {
	#[instrument(skip(self))]
	fn vsce_package(&self) -> Result<()> {
		info!("Packaging extension into a .vsix with vsce");
		let pnpm = resolve_program_in_path("pnpm")
			.context("Failed to find pnpm in PATH")?;
		let mut cmd = process::Command::new(pnpm);
		cmd.arg("--filter")
			.arg("vencord-user-companion")
			.arg("exec")
			.arg("vsce")
			.arg("package")
			.arg("--no-dependencies");
		cmd.run()
			.context("Failed to package VSCode extension")
	}
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		info!(?self, "Packaging VSCode extension");
		// Build the LSP binary and bundle the client before packaging so the
		// staged bin/ and dist/ are up to date in the resulting .vsix.
		build::extension::Command { dev: self.dev }.run()?;
		self.vsce_package()?;
		info!("Done");
		Ok(())
	}
}
