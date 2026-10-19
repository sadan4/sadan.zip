mod delta;

use anyhow::{Context, Result, anyhow, ensure};
use explorer_types::{Channel, FullBundle, intl::IntlMessages};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
	cmp::Ordering,
	collections::BTreeMap,
	env,
	fmt::Debug,
	fs,
	io::{self, BufReader, BufWriter, IntoInnerError},
	num::NonZero,
	ops::Bound,
	path::{Path, PathBuf},
	thread,
};

pub use delta::{
	DeltaHeader,
	hash_base,
	read_delta_body,
	read_delta_file,
	read_delta_header,
	read_delta_header_file,
	write_delta,
};

pub const DATA_FILE_NAME: &str = "data.mpk.zst";
/// A [`DATA_FILE_NAME`] stored as a delta against another build's data, see
/// [`write_full_bundle_delta`]
pub const DATA_DELTA_FILE_NAME: &str = "data.delta.zst";
pub const METADATA_FILE_NAME: &str = "meta.mpk.zst";
/// <100b compressed
pub const METADATA_ZSTD_LEVEL: i32 = 0;
/// 150-200mb uncompressed
pub const DATA_ZSTD_LEVEL: i32 = 10;
pub const INTL_FILE_NAME: &str = "intl.mpk.zst";
/// ~1.5mb uncompressed
pub const INTL_ZSTD_LEVEL: i32 = 10;

const BUF_SIZE: usize = 1024 * 1024;
const ZSTD_MAX_WORKERS: usize = 16;
/// first of the 16 zstd skippable frame magic numbers
const SKIPPABLE_FRAME_MAGIC: u32 = 0x184D_2A50;
/// largest window zstd supports on 64 bit targets
const ZSTD_MAX_WINDOW_LOG: u32 = 31;

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

/// Whether the build directory has a data file, either full or a delta
pub fn build_has_data(build_dir: &Path) -> bool {
	build_dir.join(DATA_FILE_NAME).is_file()
		|| build_dir
			.join(DATA_DELTA_FILE_NAME)
			.is_file()
}

/// Whether the build directory has a full data file, which other builds can
/// be stored as deltas against
pub fn build_has_full_data(build_dir: &Path) -> bool {
	build_dir.join(DATA_FILE_NAME).is_file()
}

/// How many zstd workers to compress with
fn zstd_workers() -> u32 {
	let workers = thread::available_parallelism()
		.map_or(1, NonZero::get)
		.min(ZSTD_MAX_WORKERS);
	u32::try_from(workers).unwrap_or(1)
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
	// compressing a 30mb bundle takes ~2.5s on one thread, ~0.7s on 16
	enc.multithread(zstd_workers())
		.context("Failed to enable zstd workers")?;
	rmp_serde::encode::write_named(&mut enc, value)
		.context("Failed to encode msgpack")?;
	enc.finish()
		.context("Failed to finish zstd stream")
}

/// Compresses `raw` in memory with every zstd worker
pub fn compress_zst(raw: &[u8], level: i32) -> Result<Vec<u8>> {
	let mut enc = zstd::Encoder::new(Vec::new(), level)
		.context("Failed to start zstd stream")?;
	enc.multithread(zstd_workers())
		.context("Failed to enable zstd workers")?;
	enc.set_pledged_src_size(Some(u64::try_from(raw.len())?))?;
	io::Write::write_all(&mut enc, raw).context("Failed to compress")?;
	enc.finish()
		.context("Failed to finish zstd stream")
}

/// Decompresses a whole zstd file into memory
pub fn read_raw_zst_file(path: &Path) -> Result<Vec<u8>> {
	let f = fs::File::open(path)
		.with_context(|| format!("Failed to open {}", path.display()))?;
	let mut raw = Vec::new();
	zstd::Decoder::new(f)
		.and_then(|mut dec| io::Read::read_to_end(&mut dec, &mut raw))
		.with_context(|| format!("Failed to decompress {}", path.display()))?;
	Ok(raw)
}

/// The decompressed msgpack of the [`FullBundle`] in `dir`, whether it's
/// stored in full or as a delta
///
/// The base of a delta is looked up next to `dir`
pub fn read_full_bundle_raw(dir: &Path) -> Result<Vec<u8>> {
	let full_path = dir.join(DATA_FILE_NAME);
	if full_path.is_file() {
		return read_raw_zst_file(&full_path);
	}
	read_delta_file(&dir.join(DATA_DELTA_FILE_NAME), |header| {
		ensure!(
			is_valid_build_hash(&header.base_hash),
			"invalid delta base hash {:?}",
			header.base_hash
		);
		read_raw_zst_file(
			&dir.with_file_name(&header.base_hash)
				.join(DATA_FILE_NAME),
		)
		.context("Failed to read delta base")
	})
}

pub fn read_full_bundle_from_dir(dir: &Path) -> Result<FullBundle> {
	let full_path = dir.join(DATA_FILE_NAME);
	if full_path.is_file() {
		// streamed, so only the deserialized bundle is in memory
		return read_mpk_zst_file(&full_path);
	}
	let raw = read_full_bundle_raw(dir)?;
	rmp_serde::from_slice(&raw).context("Failed to decode msgpack")
}

/// Build hashes are hex, which also makes them safe to join onto a path
pub fn is_valid_build_hash(build_hash: &str) -> bool {
	!build_hash.is_empty()
		&& build_hash
			.chars()
			.all(|c| c.is_ascii_hexdigit())
}

pub fn read_intl_messages_from_dir(dir: &Path) -> Result<IntlMessages> {
	read_mpk_zst_file(&dir.join(INTL_FILE_NAME))
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
	// the full file wins when both exist, so this only frees space
	remove_if_exists(&build_path.join(DATA_DELTA_FILE_NAME))?;

	Ok(())
}

fn remove_if_exists(path: &Path) -> Result<()> {
	match fs::remove_file(path) {
		Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e)
			.with_context(|| format!("Failed to remove {}", path.display())),
		_ => Ok(()),
	}
}

/// Write `bundle` as a delta against the full data of `base_hash`, which
/// must not be rewritten while any delta against it exists
///
/// Falls back to [`write_full_bundle`] when the delta doesn't save much.
/// Returns whether a delta was written.
pub fn write_full_bundle_delta(
	bundle: &FullBundle,
	base_hash: &str,
) -> Result<bool> {
	let base_raw =
		read_raw_zst_file(&get_build_path(base_hash)?.join(DATA_FILE_NAME))
			.context("Failed to read delta base")?;
	write_full_bundle_delta_with_base(bundle, base_hash, &base_raw)
}

/// [`write_full_bundle_delta`] with the base's data already decompressed
pub fn write_full_bundle_delta_with_base(
	bundle: &FullBundle,
	base_hash: &str,
	base_raw: &[u8],
) -> Result<bool> {
	ensure!(
		base_hash != bundle.metadata.build_hash,
		"build {base_hash} can't be a delta against itself"
	);
	let raw =
		rmp_serde::to_vec_named(bundle).context("Failed to encode msgpack")?;
	let delta =
		write_delta(Vec::new(), &raw, base_hash, base_raw, DATA_ZSTD_LEVEL)?;
	// a full data file is ~1/5 of the raw size, a useful delta is far
	// smaller than that
	if delta.len() > raw.len() / 10 {
		drop(raw);
		write_full_bundle(bundle)?;
		return Ok(false);
	}

	let build_path = get_build_path(&bundle.metadata.build_hash)?;
	if !build_path.exists() {
		fs::create_dir_all(&build_path)?;
	}
	write_mpk_zst_atomic(
		&build_path.join(METADATA_FILE_NAME),
		&bundle.metadata,
		METADATA_ZSTD_LEVEL,
	)?;
	let delta_path = build_path.join(DATA_DELTA_FILE_NAME);
	write_atomic_with(&delta_path, |w| {
		io::Write::write_all(w, &delta)?;
		Ok(())
	})?;
	drop(delta);

	// this may be about to delete the only other copy of the data, so make
	// sure what landed on disk decodes
	let verify = || -> Result<()> {
		let mut reader = BufReader::new(fs::File::open(&delta_path)?);
		let header = read_delta_header(&mut reader)?;
		let back = read_delta_body(reader, &header, base_raw)?;
		ensure!(back == raw, "decoded to different data than was written");
		Ok(())
	};
	if let Err(e) = verify() {
		remove_if_exists(&delta_path)?;
		return Err(
			e.context(format!("Failed to verify {}", delta_path.display()))
		);
	}
	drop(raw);

	// the full file wins when both exist, remove it so the delta is used
	remove_if_exists(&build_path.join(DATA_FILE_NAME))?;

	Ok(true)
}

/// Write the intl messages for `build_hash` next to its data file
///
/// Call this before [`write_full_bundle`], so any build with a data file also
/// has its intl messages
pub fn write_intl_messages(
	build_hash: &str,
	intl: &IntlMessages,
) -> Result<()> {
	let build_path = get_build_path(build_hash)?;

	if !build_path.exists() {
		fs::create_dir_all(&build_path)?;
	}

	write_mpk_zst_atomic(
		&build_path.join(INTL_FILE_NAME),
		intl,
		INTL_ZSTD_LEVEL,
	)
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
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path().join("round_trip.mpk.zst");
		let value = vec![("a".to_owned(), 1u32), ("b".to_owned(), 2)];
		write_mpk_zst_atomic(&path, &value, METADATA_ZSTD_LEVEL).unwrap();
		let back: Vec<(String, u32)> = read_mpk_zst_file(&path).unwrap();
		assert_eq!(value, back);
		// the temp file must not be left behind
		assert!(!path.with_extension("zst.tmp").exists());
	}

	#[test]
	fn intl_messages_round_trip() {
		use explorer_types::{ModuleId, intl::IntlMessage};
		use serde_json::json;
		use smol_str::SmolStr;

		let dir = tempfile::tempdir().unwrap();
		let intl = IntlMessages {
			modules: vec![ModuleId(1), ModuleId(2)],
			messages: [
				(
					SmolStr::new_static("+Qr6vP"),
					IntlMessage {
						module: ModuleId(1),
						value: json!(["Hello, ", [1, "name"], "!"]),
					},
				),
				(
					// longer than SmolStr's inline capacity
					SmolStr::new("not_a_hashed_key_but_still_valid"),
					IntlMessage {
						module: ModuleId(2),
						value: json!([[8, "$b", ["bold"], null]]),
					},
				),
			]
			.into(),
		};
		write_mpk_zst_atomic(
			&dir.path().join(INTL_FILE_NAME),
			&intl,
			INTL_ZSTD_LEVEL,
		)
		.unwrap();
		let back = read_intl_messages_from_dir(dir.path()).unwrap();
		assert_eq!(intl, back);
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
