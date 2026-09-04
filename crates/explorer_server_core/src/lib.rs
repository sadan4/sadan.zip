use anyhow::{Context, Result, anyhow};
use explorer_types::FullBundle;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
	cmp::Ordering,
	collections::BTreeMap,
	env,
	fmt::Debug,
	fs,
	io::{self, BufReader, BufWriter, IntoInnerError},
	ops::Bound,
	path::{Path, PathBuf},
};

pub const DATA_FILE_NAME: &str = "data.mpk.zst";
pub const METADATA_FILE_NAME: &str = "meta.mpk.zst";
/// <100b compressed
pub const METADATA_ZSTD_LEVEL: i32 = 0;
/// 150-200mb uncompressed
pub const DATA_ZSTD_LEVEL: i32 = 10;

const BUF_SIZE: usize = 1024 * 1024;

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

/// Deserializes a zstd compressed msgpack value straight out of `reader`,
/// never holding the decompressed bytes in memory.
pub fn read_mpk_zst<T, R>(reader: R) -> Result<T>
where
	T: DeserializeOwned,
	R: io::Read,
{
	let raw_zst =
		zstd::Decoder::new(reader).context("Failed to read zstd stream")?;
	rmp_serde::from_read(BufReader::with_capacity(BUF_SIZE, raw_zst))
		.context("Failed to decode msgpack")
}

/// [`read_mpk_zst`] over a file, streaming it off the disk.
pub fn read_mpk_zst_file<T>(path: &Path) -> Result<T>
where
	T: DeserializeOwned,
{
	let f = fs::File::open(path)
		.with_context(|| format!("Failed to open {}", path.display()))?;
	read_mpk_zst(f)
		.with_context(|| format!("Failed to read {}", path.display()))
}

/// Serializes `value` as msgpack directly into a zstd stream on `writer`,
/// never holding either encoding in memory.
pub fn write_mpk_zst<T, W>(writer: W, value: &T, level: i32) -> Result<W>
where
	T: Serialize,
	W: io::Write,
{
	let mut enc = zstd::Encoder::new(writer, level)
		.context("Failed to start zstd stream")?;
	rmp_serde::encode::write_named(&mut enc, value)
		.context("Failed to encode msgpack")?;
	enc.finish()
		.context("Failed to finish zstd stream")
}

pub fn read_full_bundle_from_dir(dir: &Path) -> Result<FullBundle> {
	read_mpk_zst_file(&dir.join(DATA_FILE_NAME))
}

/// Runs `f` against a temp file next to `path`, then renames it over `path`.
fn write_atomic_with<F>(path: &Path, f: F) -> Result<()>
where
	F: FnOnce(&mut BufWriter<fs::File>) -> Result<()>,
{
	// same directory so the rename stays on one filesystem
	let mut tmp_name = path
		.file_name()
		.ok_or_else(|| anyhow!("{} has no file name", path.display()))?
		.to_owned();
	tmp_name.push(".tmp");
	let tmp_path = path.with_file_name(tmp_name);

	let file = fs::File::create(&tmp_path).with_context(|| {
		format!("Failed to create temp file {}", tmp_path.display())
	})?;
	let mut w = BufWriter::with_capacity(BUF_SIZE, file);
	f(&mut w).with_context(|| {
		format!("Failed to write temp file {}", tmp_path.display())
	})?;
	let file = w
		.into_inner()
		.map_err(IntoInnerError::into_error)
		.with_context(|| {
			format!("Failed to flush temp file {}", tmp_path.display())
		})?;
	file.sync_all().with_context(|| {
		format!("Failed to sync temp file {}", tmp_path.display())
	})?;
	drop(file);

	fs::rename(&tmp_path, path).with_context(|| {
		format!(
			"Failed to rename {} to {}",
			tmp_path.display(),
			path.display()
		)
	})?;
	Ok(())
}

/// [`write_mpk_zst`] into `path`, atomically.
pub fn write_mpk_zst_atomic<T>(path: &Path, value: &T, level: i32) -> Result<()>
where
	T: Serialize,
{
	write_atomic_with(path, |w| {
		write_mpk_zst(w, value, level)?;
		Ok(())
	})
}

pub fn write_full_bundle(bundle: &FullBundle) -> Result<()> {
	let build_path = get_build_path(&bundle.metadata.build_hash)?;

	if !build_path.exists() {
		fs::create_dir_all(&build_path)?;
	}

	write_mpk_zst_atomic(
		&build_path.join(METADATA_FILE_NAME),
		&bundle.metadata,
		METADATA_ZSTD_LEVEL,
	)?;
	write_mpk_zst_atomic(
		&build_path.join(DATA_FILE_NAME),
		bundle,
		DATA_ZSTD_LEVEL,
	)?;

	Ok(())
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
	fn mpk_zst_round_trip() {
		let value = EncodableBuild {
			channel: Channel::Canary,
			build_hash: "deadbeef".to_owned(),
			web_js_url: "/assets/web.js".to_owned(),
			global_env_text: "{}".repeat(4096),
		};
		let mut buf = Vec::new();
		write_mpk_zst(&mut buf, &value, DATA_ZSTD_LEVEL).unwrap();
		let back: EncodableBuild = read_mpk_zst(buf.as_slice()).unwrap();
		assert_eq!(value, back);
	}

	#[test]
	fn mpk_zst_atomic_round_trip() {
		let dir = env::temp_dir().join("explorer_server_core_atomic_test");
		fs::create_dir_all(&dir).unwrap();
		let path = dir.join("round_trip.mpk.zst");
		let value = vec![("a".to_owned(), 1u32), ("b".to_owned(), 2)];
		write_mpk_zst_atomic(&path, &value, METADATA_ZSTD_LEVEL).unwrap();
		let back: Vec<(String, u32)> = read_mpk_zst_file(&path).unwrap();
		assert_eq!(value, back);
		// the temp file must not be left behind
		assert!(!path.with_extension("zst.tmp").exists());
		fs::remove_file(&path).unwrap();
	}

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
