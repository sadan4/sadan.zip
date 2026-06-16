use std::{path::PathBuf, process};

use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	util::{
		cmd::{CommandExt as _, resolve_program_in_path},
		fs,
	},
};

#[derive(Args, Debug)]
pub struct Command {
	/// Build the extension in development mode (skip minification, keep
	/// readable output). Also passed through to cargo as a debug build.
	#[arg(long, default_value_t = false)]
	pub dev: bool,
}

const BIN_NAME: &str = if cfg!(windows) {
	"companion_lsp.exe"
} else {
	"companion_lsp"
};

impl Command {
	fn cargo_bin_path(&self) -> PathBuf {
		let profile = if self.dev { "debug" } else { "release" };
		PathBuf::from("target")
			.join(profile)
			.join(BIN_NAME)
	}

	fn extension_bin_path() -> PathBuf {
		PathBuf::from("packages")
			.join("VencordCompanion")
			.join("bin")
			.join(BIN_NAME)
	}

	#[instrument(skip(self))]
	fn build_lsp(&self) -> Result<()> {
		info!("Building companion_lsp binary");
		process::Command::cargo("build")?
			.arg("-p")
			.arg("companion_lsp")
			.arg_if(!self.dev, "--release")
			.run()
			.context("Failed to build companion_lsp")
	}

	#[instrument(skip(self))]
	fn stage_lsp_binary(&self) -> Result<()> {
		let src = self.cargo_bin_path();
		let dst = Self::extension_bin_path();
		info!("Staging {} -> {}", src.display(), dst.display());
		let bin_dir = dst.parent().expect("bin path has a parent");
		fs::create_dir_all(bin_dir).with_context(|| {
			format!("Failed to create {}", bin_dir.display())
		})?;
		fs::rm_if_exists(&dst)?;
		fs::copy(&src, &dst).with_context(|| {
			format!(
				"Failed to copy {} -> {}",
				src.display(),
				dst.display(),
			)
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
