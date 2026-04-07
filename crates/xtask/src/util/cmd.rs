use std::{
	env,
	ffi::OsStr,
	path::{Path, PathBuf},
	process,
};

use anyhow::{Context, Result, anyhow, bail};
use tracing::{debug, instrument, trace, warn};

use crate::deps::pnpm_i;
pub trait CommandExt {
	fn run(&mut self) -> Result<()>;
	fn npx(program: &str) -> Result<Self>
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

#[cfg(not(windows))]
const PATH_DELIMITER: char = ':';

#[cfg(windows)]
const PATH_DELIMITER: char = ';';

fn resolve_program_in_path(file: impl AsRef<OsStr>) -> Result<PathBuf> {
	let file = Path::new(&file);
	if file.is_absolute() {
		if !file.exists() {
			bail!("Program {} does not exist", file.display());
		}
		eprintln!("absolute path {} exists", file.display());
		return Ok(file.to_owned());
	}

	let env_path = env::var("PATH")?;
	let env_path = env_path.split(PATH_DELIMITER);
	if cfg!(windows) && file.extension().is_none() {
		let search_path_refs = env_path.map(Path::new);
		let path_exts = env::var("PATHEXT")?;
		let paths = path_exts
			.split(PATH_DELIMITER)
			.map(|ext| ext.trim_start_matches('.'))
			.map(|ext| file.with_extension(ext))
			.flat_map(|file_name| {
				search_path_refs
					.clone()
					.map(move |dir| dir.join(&file_name))
			});
		for path in paths {
			dbg!(&path);
			if path.exists() {
				eprintln!(
					"found {} in PATH at {}",
					file.display(),
					path.display()
				);
				return Ok(path);
			}
		}
	} else {
		let paths = env_path
			.map(Path::new)
			.map(|dir| dir.join(file));
		for path in paths {
			if path.exists() {
				eprintln!(
					"found {} in PATH at {}",
					file.display(),
					path.display()
				);
				return Ok(path);
			}
		}
	}

	bail!("Could not find {} in PATH", file.display());
}
