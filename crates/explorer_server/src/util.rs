use anyhow::Result;
use std::{env, fs, path::PathBuf};

pub fn get_root_build_path() -> Result<PathBuf> {
    let build_path = env::current_dir()?.join("builds");
    if !build_path.exists() {
        fs::create_dir_all(&build_path)?;
    }
    Ok(build_path)
}

pub fn get_build_path(build_hash: &str) -> Result<PathBuf> {
    Ok(get_root_build_path()?.join(build_hash))
}

pub fn is_build_downloaded(build_hash: &str) -> Result<bool> {
    Ok(get_build_path(build_hash)?.is_dir())
}
