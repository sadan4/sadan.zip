use std::io;

use anyhow::{Context as _, Result};
use serde::de::DeserializeOwned;

pub fn read_struct<T: DeserializeOwned>(from: impl io::Read) -> Result<T> {
    let mpk_raw_data = zstd::decode_all(from).context("failed to decompress struct")?;
    let data: T = rmp_serde::from_slice(&mpk_raw_data).context("failed to deserialize struct")?;
    Ok(data)
}
