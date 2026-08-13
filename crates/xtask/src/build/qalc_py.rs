use std::{
	env,
	path::{Path, PathBuf},
	process,
};

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
pub const CONTEXT: &str = "crates/qalc_sbox_py";
/// Target dir used inside the container, kept separate from the host's
/// nix-linked `target/` so the two never clobber each other.
const CONTAINER_TARGET: &str = "target/docker";
/// Cargo package to build.
const PACKAGE: &str = "qalc_sbox_py";
/// `cdylib` output name cargo emits (`lib` + crate name).
const LIB_NAME: &str = "libqalc_sbox_py.so";
/// Name the module must have for `import qalc_sbox_py` to work.
const MODULE_NAME: &str = "qalc_sbox_py.so";
/// Cargo bin target that emits the `.pyi` stub via `pyo3-stub-gen`.
const STUB_BIN: &str = "stub_gen";
/// Package tree `stub_gen` writes, relative to the crate dir. Mixed layout
/// (`python-source = "python"` in `pyproject.toml`) puts the nested
/// `qalc_sbox_py.qalc_sandbox` stubs under here.
const STUB_PKG: &str = "python/qalc_sbox_py";

#[derive(Args, Debug)]
pub struct Command {
	/// Build in debug mode instead of the default release profile.
	#[arg(long, default_value_t = false)]
	pub(crate) debug: bool,
	/// Directory the built module is copied into.
	#[arg(long, default_value = "crates/qalc_sbox_py/dist")]
	pub(crate) out: PathBuf,
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
			.arg(CONTEXT)
			.run()
			.with_context(
				|| "docker build failed (is docker installed and running?)",
			)
	}

	/// A `docker run` invocation into the builder image, set up with the
	/// workspace bind-mount and the container-local `target/`/`CARGO_HOME`.
	/// The in-container argv is appended by the caller.
	pub(crate) fn container_cmd() -> Result<process::Command> {
		let root = env::current_dir()?;
		let root = root
			.to_str()
			.with_context(|| "workspace path is not valid UTF-8")?;
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
			// `stub_gen` (via `pyo3-stub-gen`) reads `CARGO_MANIFEST_DIR` at
			// *runtime* — eagerly, in an `unwrap_or` arg — so it panics when the
			// binary is run directly (no cargo) with the var unset. `cargo build`
			// overrides this per-crate, so setting it here is safe for all uses.
			.args(["-e", &format!("CARGO_MANIFEST_DIR=/work/{CONTEXT}")])
			.arg(IMAGE_TAG);
		Ok(cmd)
	}

	#[instrument(skip(self))]
	fn build_module(&self) -> Result<()> {
		info!(
			"Building {PACKAGE} in container ({} profile)",
			self.profile()
		);
		// `extension-module` must be on here (no libpython linkage in the
		// `.so`) but OFF for `stub_gen` below, so build the lib on its own.
		let mut cmd = Self::container_cmd()?;
		cmd.args([
			"cargo",
			"build",
			"-p",
			PACKAGE,
			"--lib",
			"--features",
			"extension-module",
		])
		.arg_if(!self.debug, "--release");
		cmd.run()
			.with_context(|| "cargo build inside container failed")
	}

	/// Build and run the `stub_gen` bin in the container, emitting the `.pyi`.
	/// Built without `extension-module` so it links libpython and can run.
	#[instrument(skip(self))]
	fn gen_stub(&self) -> Result<()> {
		info!(
			"Building {STUB_BIN} in container ({} profile)",
			self.profile()
		);
		let mut build = Self::container_cmd()?;
		build
			.args(["cargo", "build", "-p", PACKAGE, "--bin", STUB_BIN])
			.arg_if(!self.debug, "--release");
		build.run().with_context(
			|| "cargo build --bin stub_gen inside container failed",
		)?;

		info!("Generating stubs under {STUB_PKG}");
		let bin = format!("{CONTAINER_TARGET}/{}/{STUB_BIN}", self.profile());
		let mut run = Self::container_cmd()?;
		run.arg(bin);
		run.run()
			.with_context(|| "running stub_gen inside container failed")
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

	/// Copy the generated stub package tree into the output dir, next to the
	/// module, as `qalc_sbox_py/` (`__init__.pyi` + `qalc_sandbox/`).
	#[instrument(skip(self))]
	fn stage_stub(&self) -> Result<PathBuf> {
		let src = PathBuf::from(CONTEXT).join(STUB_PKG);
		if !src.exists() {
			bail!("expected stub package {} not found", src.display());
		}
		let dst = self.out.join("qalc_sbox_py");
		fs::rm_rf_if_exists(&dst)?;
		copy_tree(&src, &dst).with_context(|| {
			format!("failed to copy {} -> {}", src.display(), dst.display())
		})?;
		Ok(dst)
	}
}

/// Recursively copy `src` into `dst`, creating dirs as needed.
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
	fs::create_dir_all(dst)?;
	for entry in std::fs::read_dir(src)? {
		let entry = entry?;
		let from = entry.path();
		let to = dst.join(entry.file_name());
		if entry.file_type()?.is_dir() {
			copy_tree(&from, &to)?;
		} else {
			fs::copy(&from, &to)?;
		}
	}
	Ok(())
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		info!(?self, "Building qalc_sbox_py");
		Self::build_image()?;
		self.build_module()?;
		let out = self.stage_module()?;
		self.gen_stub()?;
		let stub = self.stage_stub()?;
		info!(
			"Done. module at `{}`, stub at `{}`",
			out.display(),
			stub.display()
		);
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
