use std::{env, io, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info};

const REPORTER_CACHE_DIR_ENV: &str = "REPORTER_CACHE_DIR";
const XDG_CACHE_ENV: &str = "XDG_CACHE_HOME";
const CACHE_SUBDIR: &str = env!("CARGO_CRATE_NAME");

/// get the system cache dir
///
/// eg: `~/.cache/crate_name/`
///
/// the returned path will always exist and be a directory
async fn get_cache_dir() -> Result<PathBuf> {
	let mut cache_base =
		if let Some(cache_dir) = env::var_os(REPORTER_CACHE_DIR_ENV) {
			debug!("Using cache dir from env: {cache_dir:?}");
			PathBuf::from(cache_dir)
		} else if let Some(xdg_cache_dir) = env::var_os(XDG_CACHE_ENV) {
			debug!("using cache dir from {XDG_CACHE_ENV}: {xdg_cache_dir:?}");
			PathBuf::from(xdg_cache_dir)
		} else {
			debug!("using ~/.cache as default cache dir");
			env::home_dir()
				.context("Failed to get home dir")?
				.join(".cache")
		};
	match fs::metadata(&cache_base).await {
		Ok(meta) => {
			if !meta.is_dir() {
				bail!(
					"Cache base dir {} is a file, expected a directory",
					cache_base.display()
				);
			}
		}
		Err(e) => {
			if e.kind() == io::ErrorKind::NotFound {
				info!(
					"Cache base dir {cache_base:?} does not exist, creating it"
				);
				fs::create_dir_all(&cache_base)
					.await
					.context("Failed to create cache base dir")?;
			} else {
				return Err(e).context("Failed to access cache base dir");
			}
		}
	}
	cache_base.push(CACHE_SUBDIR);
	let cache_dir = cache_base;
	match fs::create_dir(&cache_dir).await {
		Ok(()) => {}
		Err(e) => {
			if e.kind() == io::ErrorKind::AlreadyExists {
				// noop
			} else {
				return Err(e).context("Failed to create cache dir");
			}
		}
	}
	Ok(cache_dir)
}

/// read a value from cache
pub async fn read<T>(key: &str) -> Result<Option<T>>
where
	T: DeserializeOwned,
{
	let cache_dir = get_cache_dir().await?;
	let cache_file = cache_dir.join(key);
	if !fs::try_exists(&cache_file)
		.await
		.context("Failed to stat cache file")?
	{
		return Ok(None);
	}
	let raw_zstd_data = fs::read(&cache_file)
		.await
		.context("Failed to read cache file")?;
	let raw_data = zstd::decode_all(&*raw_zstd_data)
		.context("Failed to decompress cache file")?;
	let data = rmp_serde::from_slice(&raw_data)
		.context("Failed to deserialize cache file")?;
	Ok(Some(data))
}

/// write a value to cache
/// 
/// it will be compressed with zstd at the given compression level (default 10)
pub async fn write<T>(
	key: &str,
	data: &T,
	compression_level: impl Into<Option<i32>>,
) -> Result<()>
where
	T: Serialize,
{
	let compression_level = compression_level.into().unwrap_or(10);
	let cache_dir = get_cache_dir()
		.await
		.context("Failed to get cache dir")?;
	let cache_file = cache_dir.join(key);
	let raw_data =
		rmp_serde::to_vec(data).context("Failed to serialize cache data")?;
	let raw_zstd_data = zstd::encode_all(&*raw_data, compression_level)
		.context("Failed to compress data")?;
	let mut file = fs::File::options()
		.write(true)
		.create(true)
		.truncate(true)
		.append(false)
		.open(&cache_file)
		.await
		.context("Failed to open cache file")?;
	file.write_all(&raw_zstd_data)
		.await
		.context("Failed to write data to file")?;
	file.flush()
		.await
		.context("Failed to flush cache file")?;
	drop(file);
	Ok(())
}
