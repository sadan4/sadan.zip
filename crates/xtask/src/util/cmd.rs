use std::{
	env,
	ffi::OsStr,
	path::{Path, PathBuf},
	process,
};

use anyhow::{Context, Result, anyhow, bail};
use stdx::process::cmd::resolve_program_in_path;
use tracing::{debug, instrument, trace, warn};

use crate::deps::pnpm_i;
pub trait CommandExt {
	fn run(&mut self) -> Result<()>;
	fn npx(program: &str) -> Result<Self>
	where
		Self: Sized;
	fn tsx(script_file: impl AsRef<OsStr>) -> Self
	where
		Self: Sized;
	fn cargo(sub_cmd: &str) -> Result<Self>
	where
		Self: Sized;
	fn arg_if(&mut self, cond: bool, arg: &str) -> &mut Self;
}

impl CommandExt for process::Command {
	#[instrument]
	fn run(&mut self) -> Result<()> {
		debug!("Running command");
		let status = self
			.status()
			.with_context(|| format!("Failed to execute command {self:?}"))?;
		if !status.success() {
			Err(anyhow!("Command failed with status {status}")).with_context(
				|| format!("Failed to execute command {self:?}"),
			)?;
		}
		Ok(())
	}

	#[instrument]
	fn npx(program: &str) -> Result<Self> {
		fn npx_(
			program: &str,
			node_modules: &Path,
		) -> Result<process::Command> {
			let program_path = if cfg!(windows) {
				format!("{program}.cmd")
			} else {
				program.to_string()
			};
			let program_path = node_modules.join(program_path);
			let final_path = if program_path.exists() {
				program_path
			} else {
				warn!(
					"{program} not found in node_modules/.bin, falling back to searching PATH"
				);
				resolve_program_in_path(program)?
			};
			Ok(process::Command::new(final_path))
		}
		let node_modules = Path::new("node_modules").join(".bin");
		if !node_modules.exists() {
			warn!(
				"node_modules/.bin does not exist, attempting to install dependencies"
			);
			pnpm_i()?;
		}
		npx_(program, &node_modules)
			.with_context(|| format!("failed to resolve program {program}"))
	}

	#[instrument(skip(script_file), fields(script_file = ?script_file.as_ref()))]
	fn tsx(script_file: impl AsRef<OsStr>) -> Self {
		let mut ret = Self::new("bun");
		ret.arg("run");
		ret.arg(script_file);
		ret
	}

	#[instrument]
	fn cargo(sub_cmd: &str) -> Result<Self>
	where
		Self: Sized,
	{
		let cargo_path = PathBuf::from(env::var("CARGO")?)
			.canonicalize()
			.with_context(|| "Failed to canonicalize CARGO path")?;
		trace!("resolved CARGO path: {}", cargo_path.display());
		let mut ret = Self::new(cargo_path);
		ret.arg(sub_cmd);
		Ok(ret)
	}

	fn arg_if(&mut self, cond: bool, arg: &str) -> &mut Self {
		if cond {
			self.arg(arg);
		}
		self
	}
}
