use anyhow::{Context, Result, anyhow};
use explorer_types::FullBundle;
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	collections::BTreeMap,
	env,
	fmt::Debug,
	fs,
	io::{self, Write as _},
	ops::Bound,
	path::{Path, PathBuf},
};

pub const DATA_FILE_NAME: &str = "data.mpk.zst";
pub const METADATA_FILE_NAME: &str = "meta.mpk.zst";

pub fn get_root_build_path() -> Result<PathBuf> {
	let build_path = env::current_dir()
		.context("Failed to get current dir")?
		.join("builds");
	if !build_path.exists() {
		fs::create_dir_all(&build_path)
			.context("Failed to create root build path")?;
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

pub fn read_full_bundle_from_dir(dir: &Path) -> Result<FullBundle> {
	let data_path = dir.join(DATA_FILE_NAME);
	let f = fs::File::open(&data_path).with_context(|| {
		format!("Failed to open bundle data at {}", data_path.display())
	})?;
	let raw_zst = zstd::Decoder::new(f).with_context(|| {
		format!("Failed to read zstd stream at {}", data_path.display())
	})?;
	let bundle = rmp_serde::from_read(raw_zst).with_context(|| {
		format!("Failed to decode bundle at {}", data_path.display())
	})?;
	Ok(bundle)
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
	// same directory so the rename stays on one filesystem
	let mut tmp_name = path
		.file_name()
		.ok_or_else(|| anyhow!("{} has no file name", path.display()))?
		.to_owned();
	tmp_name.push(".tmp");
	let tmp_path = path.with_file_name(tmp_name);

	let mut f = fs::File::create(&tmp_path).with_context(|| {
		format!("Failed to create temp file {}", tmp_path.display())
	})?;
	f.write_all(contents).with_context(|| {
		format!("Failed to write temp file {}", tmp_path.display())
	})?;
	f.sync_all().with_context(|| {
		format!("Failed to sync temp file {}", tmp_path.display())
	})?;
	drop(f);

	fs::rename(&tmp_path, path).with_context(|| {
		format!(
			"Failed to rename {} to {}",
			tmp_path.display(),
			path.display()
		)
	})?;
	Ok(())
}

pub fn write_full_bundle(bundle: &FullBundle) -> Result<()> {
	let build_path = get_build_path(&bundle.metadata.build_hash)?;

	if !build_path.exists() {
		fs::create_dir_all(&build_path)?;
	}

	let meta_bin = rmp_serde::to_vec_named(&bundle.metadata)?;
	let meta_zst = zstd::encode_all(meta_bin.as_slice(), 0)?;
	drop(meta_bin);
	write_atomic(&build_path.join(METADATA_FILE_NAME), &meta_zst)?;
	drop(meta_zst);
	let data_mpk = rmp_serde::to_vec_named(&bundle)?;
	let data_zst = compress_full_bundle_data(&data_mpk)?;
	drop(data_mpk);

	write_atomic(&build_path.join(DATA_FILE_NAME), &data_zst)?;

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
	let upper_bound = map
		.range((Bound::Excluded(key), Bound::Unbounded))
		.next();
	(lower_bound, upper_bound)
}

/// It is a logic error for `arr` to not be sorted
pub fn get_around_arr<F, T>(arr: &[T], f: F) -> (Option<usize>, Option<usize>)
where
	F: Fn(&T) -> Ordering,
{
	match arr.binary_search_by(f) {
		Ok(idx) => {
			if idx == 0 {
				(None, Some(1))
			} else if idx == arr.len() - 1 {
				(Some(idx - 1), None)
			} else {
				(Some(idx - 1), Some(idx + 1))
			}
		}
		Err(idx) => {
			if arr.is_empty() {
				(None, None)
			} else if idx == 0 {
				(None, Some(0))
			} else if idx == arr.len() {
				(Some(idx - 1), None)
			} else {
				(Some(idx - 1), Some(idx))
			}
		}
	}
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

	#[test]
	fn test_get_around_arr() {
		let arr = [1, 2, 4, 5];
		let arr2 = [1, 2, 3, 4, 5];

		let (b, a) = get_around_arr(&arr, |x| x.cmp(&3));
		let (b2, a2) = get_around_arr(&arr2, |x| x.cmp(&3));
		let b = b.map(|i| arr[i]);
		let a = a.map(|i| arr[i]);
		let b2 = b2.map(|i| arr2[i]);
		let a2 = a2.map(|i| arr2[i]);

		assert_eq!(b, b2);
		assert_eq!(a, a2);

		assert_eq!(b, Some(2));
		assert_eq!(a, Some(4));
	}

	#[test]
	fn get_around_empty_arr() {
		assert_eq!(get_around_arr(&[0; 0], |x| x.cmp(&3)), (None, None));
	}

	#[test]
	fn test_get_around_arr_edge_cases() {
		let arr = [4, 5];
		let arr2 = [3, 4, 5];
		let arr3 = [1, 2, 3];
		let arr4 = [1, 2];
		let (b, a) = get_around_arr(&arr, |x| x.cmp(&3));
		let (b2, a2) = get_around_arr(&arr2, |x| x.cmp(&3));
		let (b3, a3) = get_around_arr(&arr3, |x| x.cmp(&3));
		let (b4, a4) = get_around_arr(&arr4, |x| x.cmp(&3));
		let b = b.map(|i| arr[i]);
		let a = a.map(|i| arr[i]);
		let b2 = b2.map(|i| arr2[i]);
		let a2 = a2.map(|i| arr2[i]);
		let b3 = b3.map(|i| arr3[i]);
		let a3 = a3.map(|i| arr3[i]);
		let b4 = b4.map(|i| arr4[i]);
		let a4 = a4.map(|i| arr4[i]);

		assert_eq!(b, b2);
		assert_eq!(a, a2);
		assert_eq!(b, None);
		assert_eq!(a, Some(4));

		assert_eq!(b3, b4);
		assert_eq!(a3, a4);
		assert_eq!(b3, Some(2));
		assert_eq!(a3, None);
	}
}
