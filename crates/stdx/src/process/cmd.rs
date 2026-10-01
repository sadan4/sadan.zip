use std::{
	env,
	ffi::OsStr,
	path::{Path, PathBuf},
};

use anyhow::{Result, bail};

#[cfg(not(windows))]
pub const PATH_DELIMITER: char = ':';

#[cfg(windows)]
pub const PATH_DELIMITER: char = ';';

/// TODO: document
pub fn resolve_program_in_path(file: impl AsRef<OsStr>) -> Result<PathBuf> {
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
			if path.exists() {
				return Ok(path);
			}
		}
	} else {
		let paths = env_path
			.map(Path::new)
			.map(|dir| dir.join(file));
		for path in paths {
			if path.exists() {
				return Ok(path);
			}
		}
	}

	bail!("Could not find {} in PATH", file.display());
}
