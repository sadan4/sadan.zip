use std::{env, io, path::PathBuf};

use derive_more::IsVariant;
use prost::Message;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info};

const REPORTER_CACHE_DIR_ENV: &str = "REPORTER_CACHE_DIR";
const XDG_CACHE_ENV: &str = "XDG_CACHE_HOME";
const CACHE_SUBDIR: &str = env!("CARGO_CRATE_NAME");

type Result<T> = std::result::Result<T, CacheError>;

#[derive(thiserror::Error, Debug, IsVariant)]
pub enum CacheError {
	#[error("Failed to get home dir")]
	NoHomeDir,
	#[error("{} is a file. expected a directory", .0.display())]
	NotDir(PathBuf),
	#[error("Failed to create directory: {}", path.display())]
	CreateDir {
		path: PathBuf,
		#[source]
		cause: io::Error,
	},
	#[error("Failed to access path: {}", path.display())]
	Access {
		path: PathBuf,
		#[source]
		cause: io::Error,
	},
	#[error("Failed to read cache file: {}", path.display())]
	Read {
		path: PathBuf,
		#[source]
		cause: io::Error,
	},
	#[error("Failed to write cache file: {}", path.display())]
	Write {
		path: PathBuf,
		#[source]
		cause: io::Error,
	},
	#[error("ZSTD error: {0}")]
	Zstd(#[source] io::Error),
	#[error("Failed to deserialize cache file")]
	Deserialize(#[source] rmp_serde::decode::Error),
	#[error("Failed to serialize cache file")]
	Serialize(#[source] rmp_serde::encode::Error),
	#[error("Failed to decode cache file")]
	Protobuf(#[source] prost::DecodeError),
}

/// get the system cache dir
///
/// eg: `~/.cache/crate_name/`
///
/// the returned path will always exist and be a directory
async fn get_cache_dir() -> Result<PathBuf> {
	let mut cache_base =
		if let Some(cache_dir) = env::var_os(REPORTER_CACHE_DIR_ENV) {
			debug!("Using cache dir from env: {cache_dir:?}");
			return Ok(PathBuf::from(cache_dir));
		} else if let Some(xdg_cache_dir) = env::var_os(XDG_CACHE_ENV) {
			debug!("using cache dir from {XDG_CACHE_ENV}: {xdg_cache_dir:?}");
			PathBuf::from(xdg_cache_dir)
		} else {
			debug!("using ~/.cache as default cache dir");
			env::home_dir()
				.ok_or(CacheError::NoHomeDir)?
				.join(".cache")
		};
	match fs::metadata(&cache_base).await {
		Ok(meta) => {
			if !meta.is_dir() {
				return Err(CacheError::NotDir(cache_base));
			}
		}
		Err(e) => {
			if e.kind() == io::ErrorKind::NotFound {
				info!(
					"Cache base dir {cache_base:?} does not exist, creating it"
				);
				fs::create_dir_all(&cache_base)
					.await
					.map_err(|e| CacheError::CreateDir {
						// not needed but cba to not use map_err
						path: cache_base.clone(),
						cause: e,
					})?;
			} else {
				return Err(CacheError::Access {
					path: cache_base,
					cause: e,
				});
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
				return Err(CacheError::CreateDir {
					cause: e,
					path: cache_dir,
				});
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
		.map_err(|e| CacheError::Access {
			path: cache_file.clone(),
			cause: e,
		})? {
		return Ok(None);
	}
	debug!("Reading cache file: {}", cache_file.display());
	let raw_zstd_data = fs::read(&cache_file)
		.await
		.map_err(|e| CacheError::Read {
			path: cache_file.clone(),
			cause: e,
		})?;
	let raw_data =
		zstd::decode_all(&*raw_zstd_data).map_err(CacheError::Zstd)?;
	let data =
		rmp_serde::from_slice(&raw_data).map_err(CacheError::Deserialize)?;
	Ok(Some(data))
}

/// read a protobuf-encoded value from cache
pub async fn read_proto<T>(key: &str) -> Result<Option<T>>
where
	T: Message + Default,
{
	let cache_dir = get_cache_dir().await?;
	let cache_file = cache_dir.join(key);
	if !fs::try_exists(&cache_file)
		.await
		.map_err(|e| CacheError::Access {
			path: cache_file.clone(),
			cause: e,
		})? {
		return Ok(None);
	}
	debug!("Reading cache file: {}", cache_file.display());
	let raw_zstd_data = fs::read(&cache_file)
		.await
		.map_err(|e| CacheError::Read {
			path: cache_file.clone(),
			cause: e,
		})?;
	let raw_data =
		zstd::decode_all(&*raw_zstd_data).map_err(CacheError::Zstd)?;
	let data = T::decode(&*raw_data).map_err(CacheError::Protobuf)?;
	Ok(Some(data))
}

/// Invalidate a cache entry by key.
///
/// This will remove the cache file if it exists, and do nothing if it does not exist.
pub async fn invalidate(key: &str) -> Result<()> {
	let cache_dir = get_cache_dir().await?;
	let cache_file = cache_dir.join(key);
	if fs::try_exists(&cache_file)
		.await
		.map_err(|e| CacheError::Access {
			path: cache_file.clone(),
			cause: e,
		})? {
		fs::remove_file(&cache_file)
			.await
			.map_err(|e| CacheError::Access {
				cause: e,
				path: cache_file,
			})?;
	}
	Ok(())
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
	let cache_dir = get_cache_dir().await?;
	let cache_file = cache_dir.join(key);
	let raw_data = rmp_serde::to_vec(data).map_err(CacheError::Serialize)?;
	let raw_zstd_data = zstd::encode_all(&*raw_data, compression_level)
		.map_err(CacheError::Zstd)?;
	let mut file = fs::File::options()
		.write(true)
		.create(true)
		.truncate(true)
		.append(false)
		.open(&cache_file)
		.await
		.map_err(|e| CacheError::Access {
			path: cache_file.clone(),
			cause: e,
		})?;
	file.write_all(&raw_zstd_data)
		.await
		.map_err(|e| CacheError::Write {
			path: cache_file.clone(),
			cause: e,
		})?;
	file.flush()
		.await
		.map_err(|e| CacheError::Write {
			path: cache_file.clone(),
			cause: e,
		})?;
	drop(file);
	Ok(())
}

/// write a protobuf-encoded value to cache
///
/// it will be compressed with zstd at the given compression level (default 10)
pub async fn write_proto<T>(
	key: &str,
	data: &T,
	compression_level: impl Into<Option<i32>>,
) -> Result<()>
where
	T: Message,
{
	let compression_level = compression_level.into().unwrap_or(10);
	let cache_dir = get_cache_dir().await?;
	let cache_file = cache_dir.join(key);
	let raw_data = data.encode_to_vec();
	let raw_zstd_data = zstd::encode_all(&*raw_data, compression_level)
		.map_err(CacheError::Zstd)?;
	let mut file = fs::File::options()
		.write(true)
		.create(true)
		.truncate(true)
		.append(false)
		.open(&cache_file)
		.await
		.map_err(|e| CacheError::Access {
			path: cache_file.clone(),
			cause: e,
		})?;
	file.write_all(&raw_zstd_data)
		.await
		.map_err(|e| CacheError::Write {
			path: cache_file.clone(),
			cause: e,
		})?;
	file.flush()
		.await
		.map_err(|e| CacheError::Write {
			path: cache_file.clone(),
			cause: e,
		})?;
	drop(file);
	Ok(())
}
