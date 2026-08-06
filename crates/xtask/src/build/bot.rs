use std::{path::PathBuf, process};

use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	util::{cmd::CommandExt as _, fs},
};

/// The qalc sandbox worker bin name (from `crates/qalc_sbox/src/bin/`).
const WORKER_BIN: &str = "qalc_sbox_worker";
/// Where the built bot and worker are staged together.
const STAGE_DIR: &str = "crates/bot/assets";

#[derive(Args, Debug)]
pub struct Command {
	#[arg(short, long, default_value_t = false)]
	/// Build in release mode with optimizations.
	pub release: bool,
}

impl Command {
	const fn profile(&self) -> &'static str {
		if self.release { "release" } else { "debug" }
	}

	fn cargo_bin_path(&self, bin: &str) -> PathBuf {
		PathBuf::from("target")
			.join(self.profile())
			.join(bin)
	}

	fn staged_bin_path(bin: &str) -> PathBuf {
		PathBuf::from(STAGE_DIR).join(bin)
	}

	#[instrument(skip(self))]
	fn build_bins(&self) -> Result<()> {
		info!("Building bot and qalc sandbox worker");
		process::Command::cargo("build")?
			.arg("-p")
			.arg("bot")
			.arg("-p")
			.arg("qalc_sbox")
			.arg("--bin")
			.arg(WORKER_BIN)
			.arg_if(self.release, "--release")
			.run()
	}

	#[instrument(skip(self))]
	fn stage_bin(&self, bin: &str) -> Result<()> {
		let src = self.cargo_bin_path(bin);
		let dst = Self::staged_bin_path(bin);
		info!("Staging {} -> {}", src.display(), dst.display());
		fs::rm_if_exists(&dst)?;
		fs::copy(&src, &dst).with_context(|| {
			format!("Failed to copy {} -> {}", src.display(), dst.display())
		})?;
		Ok(())
	}
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		info!(?self, "Building bot");
		self.build_bins()?;
		fs::create_dir_all(STAGE_DIR)
			.with_context(|| format!("Failed to create {STAGE_DIR}"))?;
		self.stage_bin(WORKER_BIN)?;
		info!("Done. copied to `{STAGE_DIR}/{WORKER_BIN}`");
		Ok(())
	}
}
