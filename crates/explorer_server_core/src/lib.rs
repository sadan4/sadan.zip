use anyhow::{Result};
use explorer_types::FullBundle;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Debug,
    fs, io,
    path::{Path, PathBuf},
};

pub const DATA_FILE_NAME: &str = "data.mpk.zst";
pub const METADATA_FILE_NAME: &str = "meta.mpk.zst";

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

pub fn get_version_file_path() -> Result<PathBuf> {
    Ok(get_root_build_path()?.join(".ver"))
}

pub fn build_has_data(build_hash: &Path) -> bool {
    build_hash.join(DATA_FILE_NAME).is_file()
}

pub fn write_full_bundle(bundle: &FullBundle) -> Result<()> {
    let build_path = get_build_path(&bundle.metadata.build_hash)?;

    if !build_path.exists() {
        fs::create_dir_all(&build_path)?;
    }

    let meta_bin = rmp_serde::to_vec(&bundle.metadata)?;
    let meta_zst = zstd::encode_all(meta_bin.as_slice(), 0)?;
    drop(meta_bin);
    fs::write(build_path.join(METADATA_FILE_NAME), meta_zst)?;
    let data_mpk = rmp_serde::to_vec(&bundle)?;
    let data_zst = compress_full_bundle_data(&data_mpk)?;
    drop(data_mpk);

    fs::write(build_path.join(DATA_FILE_NAME), data_zst)?;

    Ok(())
}

pub fn compress_full_bundle_data(data: &[u8]) -> Result<Vec<u8>> {
    Ok(zstd::encode_all(data, 10)?)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Stable = 0,
    Canary = 1,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EncodableBuild {
    pub channel: Channel,
    pub build_hash: String,
    pub web_js_url: String,
    pub global_env_text: String,
}

impl EncodableBuild {
    pub fn encode(&self, to: &mut impl io::Write) -> Result<()> {
        rmp_serde::encode::write(to, self).map_err(From::from)
    }
    pub fn decode(from: &mut impl io::Read) -> Result<Self> {
        rmp_serde::decode::from_read(from).map_err(From::from)
    }
}
