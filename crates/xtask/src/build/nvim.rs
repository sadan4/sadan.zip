use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	build::extension,
	util::{nvim::PluginDir, target::ExtensionTarget},
};

#[derive(Args, Debug)]
pub struct Command {
	/// Build the `companion_lsp` binary in development mode (debug profile).
	#[arg(long, default_value_t = false)]
	pub dev: bool,

	/// Cross-compile `companion_lsp` for the given target. Named after the
	/// VS Code target triples (e.g. `linux-x64`, `darwin-arm64`) because the
	/// mapping to Rust triples is shared with `build extension`. When
	/// omitted, the host platform is used.
	#[arg(long)]
	pub target: Option<ExtensionTarget>,

	#[command(flatten)]
	pub plugin_dir: PluginDir,
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		let bin_dir = self.plugin_dir.resolve()?.join("bin");
		info!(?self, "Building the Neovim plugin's server binary");
		// The plugin is pure Lua, so unlike the VS Code extension there is no
		// client to bundle: staging the binary is the whole build.
		extension::Command {
			dev: self.dev,
			target: self.target,
		}
		.build_and_stage(&bin_dir)
		.context("Failed to stage companion_lsp for the Neovim plugin")?;
		info!("Done");
		Ok(())
	}
}
