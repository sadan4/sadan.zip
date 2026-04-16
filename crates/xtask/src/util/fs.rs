pub use std::fs::*;
use std::path::Path;

use anyhow::Result;

pub fn rm_if_exists(path: impl AsRef<Path>) -> Result<()> {
	let path = path.as_ref();
	if path.exists() {
		remove_file(path)?;
	}
	Ok(())
}

pub fn rm_rf_if_exists(path: impl AsRef<Path>) -> Result<()> {
	let path = path.as_ref();
	if path.exists() {
		remove_dir_all(path)?;
	}
	Ok(())
}

// pub fn dir_size(path: impl AsRef<Path>) -> Result<u64> {
// }
