use anyhow::Result;
use explorer_types::FullBundle;
use serde::{Deserialize, Serialize};
use std::{
	collections::BTreeMap,
	env,
	fmt::Debug,
	fs,
	io,
	ops::Bound,
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
	build_hash
		.join(DATA_FILE_NAME)
		.is_file()
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

impl Channel {
	pub const fn asset_base(self) -> &'static str {
		match self {
			Self::Stable => "https://discord.com/assets/",
			Self::Canary => "https://canary.discord.com/assets/",
		}
	}
	pub const fn app_base(self) -> &'static str {
		match self {
			Self::Stable => "https://discord.com/app",
			Self::Canary => "https://canary.discord.com/app",
		}
	}
}

pub fn asset_url(channel: Channel, mut path: &str) -> String {
	if path.starts_with('/') {
		path = &path[1..];
	}
	let asset_base = channel.asset_base();
	format!("{asset_base}{path}")
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

type BTreeBound<'a, K, V> = Option<(&'a K, &'a V)>;
type BTreeBounds<'a, K, V> = (BTreeBound<'a, K, V>, BTreeBound<'a, K, V>);

/// gets the bounds of the given key in the map, returning the lower and upper bounds as a tuple
/// ```
/// # use std::collections::BTreeMap;
/// # use explorer_server_core::get_around;
/// let map = BTreeMap::from([(1, 1), (2, 2), (4, 4), (5, 5)]);
/// let map2 = BTreeMap::from([(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
///
/// let (b, a) = get_around(&map, &3);
/// let (b2, a2) = get_around(&map2, &3);
///
/// assert_eq!(b, b2);
/// assert_eq!(a, a2);
///
/// assert_eq!(b, Some((&2, &2)));
/// assert_eq!(a, Some((&4, &4)));
/// ```
pub fn get_around<'m, K, V>(
	map: &'m BTreeMap<K, V>,
	key: &K,
) -> BTreeBounds<'m, K, V>
where
	K: Ord,
{
	let lower_bound = map.range(..key).next_back();
	let upper_bound = map.range((Bound::Excluded(key), Bound::Unbounded)).next();
	(lower_bound, upper_bound)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// copy of the doctest
	fn test_get_around() {
		let map = BTreeMap::from([(1, 1), (2, 2), (4, 4), (5, 5)]);
		let map2 = BTreeMap::from([(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);

		let (b, a) = get_around(&map, &3);
		let (b2, a2) = get_around(&map2, &3);

		assert_eq!(b, b2);
		assert_eq!(a, a2);

		assert_eq!(b, Some((&2, &2)));
		assert_eq!(a, Some((&4, &4)));
	}
}
