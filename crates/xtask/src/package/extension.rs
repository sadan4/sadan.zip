use std::process;

use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	build,
	util::{
		cmd::{CommandExt as _, resolve_program_in_path},
		target::ExtensionTarget,
	},
};

#[derive(Args, Debug)]
pub struct Command {
	/// Package the extension in development mode (skip minification, use a
	/// debug LSP binary). Passed through to the build step.
	#[arg(long, default_value_t = false)]
	pub dev: bool,

	/// Produce a platform-specific .vsix for the given VS Code target (e.g.
	/// `linux-x64`, `darwin-arm64`). The bundled `companion_lsp` binary is
	/// cross-compiled to the matching triple and `--target` is forwarded to
	/// `vsce package`. When omitted, a host-platform (universal) .vsix is
	/// produced.
	#[arg(long)]
	pub target: Option<ExtensionTarget>,
}

impl Command {
	#[instrument]
	fn vsce_package(target: Option<ExtensionTarget>) -> Result<()> {
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
		if let Some(target) = target {
			cmd.arg("--target").arg(target.vscode());
		}
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
		build::extension::Command {
			dev: self.dev,
			target: self.target,
		}
		.run()?;
		Self::vsce_package(self.target)?;
		info!("Done");
		Ok(())
	}
}
