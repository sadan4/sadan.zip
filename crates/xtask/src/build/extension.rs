use std::{path::PathBuf, process};

use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	util::{
		cmd::{CommandExt as _, resolve_program_in_path},
		fs,
		target::ExtensionTarget,
	},
};

#[derive(Args, Debug)]
pub struct Command {
	/// Build the extension in development mode (skip minification, keep
	/// readable output). Also passed through to cargo as a debug build.
	#[arg(long, default_value_t = false)]
	pub dev: bool,

	/// Build a platform-specific extension for the given VS Code target (e.g.
	/// `linux-x64`, `darwin-arm64`). The `companion_lsp` binary is
	/// cross-compiled to the matching Rust triple. When omitted, the host
	/// platform is used and no `--target` is passed to cargo.
	#[arg(long)]
	pub target: Option<ExtensionTarget>,
}

/// The `companion_lsp` binary filename for the host platform.
const HOST_BIN_NAME: &str = if cfg!(windows) {
	"companion_lsp.exe"
} else {
	"companion_lsp"
};

impl Command {
	/// The `companion_lsp` filename to stage: target-specific when a
	/// `--target` is given, otherwise the host binary name.
	fn bin_name(&self) -> &'static str {
		self.target
			.map_or(HOST_BIN_NAME, ExtensionTarget::bin_name)
	}

	fn cargo_bin_path(&self) -> PathBuf {
		let profile = if self.dev { "debug" } else { "release" };
		let mut path = PathBuf::from("target");
		// Cross builds land under target/<triple>/<profile>/ rather than
		// target/<profile>/.
		if let Some(target) = self.target {
			path.push(target.triple());
		}
		path.join(profile).join(self.bin_name())
	}

	fn extension_bin_path(&self) -> PathBuf {
		PathBuf::from("packages")
			.join("VencordCompanion")
			.join("bin")
			.join(self.bin_name())
	}

	#[instrument(skip(self))]
	fn build_lsp(&self) -> Result<()> {
		info!("Building companion_lsp binary");
		let mut cmd = process::Command::cargo("build")?;
		cmd.arg("-p")
			.arg("companion_lsp")
			.arg_if(!self.dev, "--release");
		if let Some(target) = self.target {
			cmd.arg("--target").arg(target.triple());
		}
		cmd.run()
			.context("Failed to build companion_lsp")
	}

	#[instrument(skip(self))]
	fn stage_lsp_binary(&self) -> Result<()> {
		let src = self.cargo_bin_path();
		let dst = self.extension_bin_path();
		info!("Staging {} -> {}", src.display(), dst.display());
		let bin_dir = dst
			.parent()
			.expect("bin path has a parent");
		fs::create_dir_all(bin_dir).with_context(|| {
			format!("Failed to create {}", bin_dir.display())
		})?;
		fs::rm_if_exists(&dst)?;
		fs::copy(&src, &dst).with_context(|| {
			format!("Failed to copy {} -> {}", src.display(), dst.display())
		})?;
		Ok(())
	}

	#[instrument(skip(self))]
	fn build_client(&self) -> Result<()> {
		info!("Building VSCode extension client");
		let pnpm = resolve_program_in_path("pnpm")
			.context("Failed to find pnpm in PATH")?;
		let mut cmd = process::Command::new(pnpm);
		cmd.arg("--filter")
			.arg("vencord-user-companion")
			.arg("run")
			.arg("build");
		if self.dev {
			cmd.arg("--").arg("--dev");
		}
		cmd.run()
			.context("Failed to build VSCode extension")
	}
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		info!(?self, "Building VSCode extension");
		self.build_lsp()?;
		self.stage_lsp_binary()?;
		self.build_client()?;
		info!("Done");
		Ok(())
	}
}
