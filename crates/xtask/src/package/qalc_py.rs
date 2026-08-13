use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

use crate::{Runnable, build, util::cmd::CommandExt as _};

/// Distribution name of the produced wheel.
const DIST_NAME: &str = "qalc_sbox_py";
/// Distribution version (kept in step with `crates/qalc_sbox_py/pyproject.toml`).
const DIST_VERSION: &str = "0.1.0";
/// Wheel-builder script, relative to the crate dir.
const WHEEL_SCRIPT: &str = "scripts/build_wheel.py";

#[derive(Args, Debug)]
pub struct Command {
	/// Build the module in debug mode. Passed through to the build step.
	#[arg(long, default_value_t = false)]
	pub debug: bool,
	/// Directory the built module/stubs live in and the wheel is written to.
	#[arg(long, default_value = "crates/qalc_sbox_py/dist")]
	pub out: PathBuf,
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		info!(?self, "Packaging qalc_sbox_py into a wheel");
		// Refresh dist/ (module + stubs) before packaging.
		build::qalc_py::Command {
			debug: self.debug,
			out: self.out.clone(),
		}
		.run()?;

		// Assemble the wheel with the builder container's own Python so the ABI
		// and platform tags match the interpreter the `.so` targets.
		let out = self
			.out
			.to_str()
			.context("output path is not valid UTF-8")?;
		let dist = format!("/work/{out}");
		let mut cmd = build::qalc_py::Command::container_cmd()?;
		cmd.args([
			"python3",
			&format!("/work/{}/{WHEEL_SCRIPT}", build::qalc_py::CONTEXT),
			"--dist",
			&dist,
			"--out",
			&dist,
			"--name",
			DIST_NAME,
			"--version",
			DIST_VERSION,
		]);
		cmd.run()
			.context("building the wheel inside the container failed")?;
		info!("Done. wheel written under `{out}`");
		Ok(())
	}
}
