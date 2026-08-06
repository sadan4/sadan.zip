use std::{env, path::PathBuf, process};

use anyhow::{Context, Result, bail};
use clap::Args;
use tracing::{info, instrument};

use crate::{
	Runnable,
	util::{cmd::CommandExt as _, fs},
};

/// Docker image tag used for the builder.
const IMAGE_TAG: &str = "sadanzip-qalc-sbox-py-builder";
/// Path to the builder Dockerfile, relative to the workspace root.
const DOCKERFILE: &str = "crates/qalc_sbox_py/Dockerfile";
/// Docker build context (kept tiny; the Dockerfile does no `COPY`).
const CONTEXT: &str = "crates/qalc_sbox_py";
/// Target dir used inside the container, kept separate from the host's
/// nix-linked `target/` so the two never clobber each other.
const CONTAINER_TARGET: &str = "target/docker";
/// Cargo package to build.
const PACKAGE: &str = "qalc_sbox_py";
/// `cdylib` output name cargo emits (`lib` + crate name).
const LIB_NAME: &str = "libqalc_sbox_py.so";
/// Name the module must have for `import qalc_sbox_py` to work.
const MODULE_NAME: &str = "qalc_sbox_py.so";

#[derive(Args, Debug)]
pub struct Command {
	/// Build in debug mode instead of the default release profile.
	#[arg(long, default_value_t = false)]
	debug: bool,
	/// Directory the built module is copied into.
	#[arg(long, default_value = "crates/qalc_sbox_py/dist")]
	out: PathBuf,
}

impl Command {
	const fn profile(&self) -> &'static str {
		if self.debug { "debug" } else { "release" }
	}

	#[instrument]
	fn build_image() -> Result<()> {
		info!("Building builder image {IMAGE_TAG}");
		process::Command::new("docker")
			.args(["build", "-t", IMAGE_TAG, "-f", DOCKERFILE])
			.arg("--build-arg")
			.arg(CONTEXT)
			.run()
			.with_context(
				|| "docker build failed (is docker installed and running?)",
			)
	}

	#[instrument(skip(self))]
	fn build_module(&self) -> Result<()> {
		let root = env::current_dir()?;
		let root = root
			.to_str()
			.with_context(|| "workspace path is not valid UTF-8")?;
		info!(
			"Building {PACKAGE} in container ({} profile)",
			self.profile()
		);
		let mut cmd = process::Command::new("docker");
		cmd.args(["run", "--rm"])
			.arg("-v")
			.arg(format!("{root}:/work"))
			.args(["-w", "/work"])
			.args(["--user", &uid_gid()?])
			.args(["-e", &format!("CARGO_TARGET_DIR=/work/{CONTAINER_TARGET}")])
			.args([
				"-e",
				&format!("CARGO_HOME=/work/{CONTAINER_TARGET}/.cargo-home"),
			])
			.arg(IMAGE_TAG)
			.args(["cargo", "build", "-p", PACKAGE])
			.arg_if(!self.debug, "--release");
		cmd.run()
			.with_context(|| "cargo build inside container failed")
	}

	#[instrument(skip(self))]
	fn stage_module(&self) -> Result<PathBuf> {
		let src = PathBuf::from(CONTAINER_TARGET)
			.join(self.profile())
			.join(LIB_NAME);
		if !src.exists() {
			bail!("expected build output {} not found", src.display());
		}
		fs::create_dir_all(&self.out).with_context(|| {
			format!("failed to create {}", self.out.display())
		})?;
		let dst = self.out.join(MODULE_NAME);
		fs::rm_if_exists(&dst)?;
		fs::copy(&src, &dst).with_context(|| {
			format!("failed to copy {} -> {}", src.display(), dst.display())
		})?;
		Ok(dst)
	}
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		info!(?self, "Building qalc_sbox_py");
		Self::build_image()?;
		self.build_module()?;
		let out = self.stage_module()?;
		info!("Done. module at `{}`", out.display());
		Ok(())
	}
}

/// `<uid>:<gid>` of the host user, so container-written files are host-owned.
fn uid_gid() -> Result<String> {
	fn read(flag: &str) -> Result<String> {
		let out = process::Command::new("id")
			.arg(flag)
			.output()?;
		if !out.status.success() {
			bail!("`id {flag}` failed");
		}
		Ok(String::from_utf8(out.stdout)?
			.trim()
			.to_string())
	}
	Ok(format!("{}:{}", read("-u")?, read("-g")?))
}
