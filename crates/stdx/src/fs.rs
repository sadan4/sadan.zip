use std::{fs, io, path::Path};

pub fn rm_if_exists(path: impl AsRef<Path>) -> io::Result<()> {
	let path = path.as_ref();
	if path.exists() {
		fs::remove_file(path)?;
	}
	Ok(())
}

pub fn rm_rf_if_exists(path: impl AsRef<Path>) -> io::Result<()> {
	let path = path.as_ref();
	if path.exists() {
		fs::remove_dir_all(path)?;
	}
	Ok(())
}
