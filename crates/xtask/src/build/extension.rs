use std::{
	path::{Path, PathBuf},
	process,
};

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

const PDB_NAME: &str = "companion_lsp.pdb";

impl Command {
	/// The `companion_lsp` filename to stage: target-specific when a
	/// `--target` is given, otherwise the host binary name.
	fn bin_name(&self) -> &'static str {
		self.target
			.map_or(HOST_BIN_NAME, ExtensionTarget::bin_name)
	}

	/// Whether the binary being built targets Windows: the requested
	/// `--target` when given, otherwise the host platform.
	fn is_windows(&self) -> bool {
		self.target
			.map_or(cfg!(windows), ExtensionTarget::is_windows)
	}

	/// The directory cargo drops artifacts in. Cross builds land under
	/// `target/<triple>/<profile>/` rather than `target/<profile>/`.
	fn cargo_out_dir(&self) -> PathBuf {
		let profile = if self.dev { "debug" } else { "release" };
		let mut path = PathBuf::from("target");
		if let Some(target) = self.target {
			path.push(target.triple());
		}
		path.join(profile)
	}

	fn cargo_bin_path(&self) -> PathBuf {
		self.cargo_out_dir()
			.join(self.bin_name())
	}

	fn extension_bin_dir() -> PathBuf {
		PathBuf::from("packages")
			.join("VencordCompanion")
			.join("bin")
	}

	/// Build `companion_lsp` and stage it (plus its `.pdb` on Windows) into
	/// `bin_dir`.
	///
	/// Shared with `cargo xtask build nvim`, which needs the same binary in a
	/// different directory and has no client to bundle.
	#[instrument(skip(self))]
	pub(in crate::build) fn build_and_stage(
		&self,
		bin_dir: &Path,
	) -> Result<()> {
		self.build_lsp()?;
		self.stage_lsp_binary_into(bin_dir)?;
		if self.is_windows() && !self.dev {
			self.stage_pdb_into(bin_dir)?;
		}
		Ok(())
	}

	#[instrument(skip(self))]
	fn build_lsp(&self) -> Result<()> {
		info!("Building companion_lsp binary");
		let mut cmd = process::Command::cargo("build")?;
		cmd.arg("-p")
			.arg("companion_lsp")
			.arg_if(!self.dev, "--release");
		if !self.dev {
			// A crash report from a user is all we get, so keep enough
			// debuginfo for symbolized backtraces without paying for full
			// DWARF. The workspace release profile strips, which would leave
			// backtraces bare.
			//
			// MSVC always emits debuginfo to a side-car `.pdb`, so `packed` is
			// the only legal setting there and `stage_pdb` ships the `.pdb`
			// next to the binary. Everywhere else `off` embeds the line tables
			// in the binary so there is nothing extra to bundle.
			let split = if self.is_windows() { "packed" } else { "off" };
			cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "line-tables-only")
				.env("CARGO_PROFILE_RELEASE_STRIP", "none")
				.env("CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO", split);
		}
		if let Some(target) = self.target {
			cmd.arg("--target").arg(target.triple());
		}
		cmd.run()
			.context("Failed to build companion_lsp")
	}

	#[instrument(skip(self))]
	fn stage_lsp_binary_into(&self, bin_dir: &Path) -> Result<()> {
		let src = self.cargo_bin_path();
		let dst = bin_dir.join(self.bin_name());
		info!("Staging {} -> {}", src.display(), dst.display());
		fs::create_dir_all(bin_dir).with_context(|| {
			format!("Failed to create {}", bin_dir.display())
		})?;
		fs::rm_if_exists(&dst)?;
		fs::copy(&src, &dst).with_context(|| {
			format!("Failed to copy {} -> {}", src.display(), dst.display())
		})?;
		Ok(())
	}

	/// Stage the MSVC `.pdb` alongside the binary so release backtraces from
	/// the shipped Windows extension resolve to symbols and line numbers. No
	/// other platform produces one: there the debuginfo lives in the binary.
	#[instrument(skip(self))]
	fn stage_pdb_into(&self, bin_dir: &Path) -> Result<()> {
		let src = self.cargo_out_dir().join(PDB_NAME);
		let dst = bin_dir.join(PDB_NAME);
		info!("Staging {} -> {}", src.display(), dst.display());
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
		// FIXME: generate extension settings and commands in parallel before building js
		self.build_and_stage(&Self::extension_bin_dir())?;
		self.build_client()?;
		info!("Done");
		Ok(())
	}
}
