//! Data stored as a zstd delta against another build's decompressed data.
//!
//! A delta file is a zstd skippable frame holding a msgpack [`DeltaHeader`],
//! followed by a single zstd frame compressed with the base's decompressed
//! bytes as its prefix (the same thing as `zstd --patch-from`).
//!
//! Consecutive builds share almost every module, so a delta is ~30x smaller
//! than compressing the build on its own.

use std::{
	fs,
	io::{BufRead, BufReader, Read, Write},
	path::Path,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh64::xxh64;

use crate::{SKIPPABLE_FRAME_MAGIC, ZSTD_MAX_WINDOW_LOG, zstd_workers};

/// headers are ~70 bytes, anything this big is a corrupt file
const MAX_HEADER_LEN: u32 = 4096;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeltaHeader {
	/// the build whose decompressed data this is a delta against
	pub base_hash: String,
	/// [`hash_base`] of the base's decompressed data when the delta was
	/// written. Rewriting the base breaks the delta, this catches that.
	pub base_xxh64: u64,
	/// length of the decompressed data
	pub raw_len: u64,
}

pub fn hash_base(base_raw: &[u8]) -> u64 {
	xxh64(base_raw, 0)
}

/// The smallest window that can reach from the end of the data back to the
/// start of the prefix
fn window_log_for(len: usize) -> u32 {
	u64::try_from(len)
		.ok()
		.and_then(u64::checked_next_power_of_two)
		.map_or(ZSTD_MAX_WINDOW_LOG, u64::ilog2)
		.clamp(10, ZSTD_MAX_WINDOW_LOG)
}

/// Writes `raw` as a delta against `base_raw`, the decompressed data of
/// `base_hash`
pub fn write_delta<W>(
	mut writer: W,
	raw: &[u8],
	base_hash: &str,
	base_raw: &[u8],
	level: i32,
) -> Result<W>
where
	W: Write,
{
	let header = DeltaHeader {
		base_hash: base_hash.to_owned(),
		base_xxh64: hash_base(base_raw),
		raw_len: u64::try_from(raw.len())?,
	};
	let header_bytes = rmp_serde::to_vec_named(&header)
		.context("Failed to encode delta header")?;
	let header_len = u32::try_from(header_bytes.len())?;
	writer.write_all(&SKIPPABLE_FRAME_MAGIC.to_le_bytes())?;
	writer.write_all(&header_len.to_le_bytes())?;
	writer.write_all(&header_bytes)?;

	let mut enc = zstd::Encoder::with_ref_prefix(writer, level, base_raw)
		.context("Failed to start zstd stream")?;
	enc.long_distance_matching(true)?;
	enc.window_log(window_log_for(base_raw.len() + raw.len()))?;
	enc.set_pledged_src_size(Some(header.raw_len))?;
	enc.include_checksum(true)?;
	enc.multithread(zstd_workers())
		.context("Failed to enable zstd workers")?;
	enc.write_all(raw)
		.context("Failed to compress delta")?;
	enc.finish()
		.context("Failed to finish zstd stream")
}

/// Reads the header at the start of a delta, leaving `reader` at the start
/// of the compressed data for [`read_delta_body`]
pub fn read_delta_header<R>(reader: &mut R) -> Result<DeltaHeader>
where
	R: Read,
{
	let mut word = [0; 4];
	reader
		.read_exact(&mut word)
		.context("Failed to read delta magic")?;
	ensure!(
		u32::from_le_bytes(word) == SKIPPABLE_FRAME_MAGIC,
		"not a delta file, bad magic"
	);
	reader
		.read_exact(&mut word)
		.context("Failed to read delta header length")?;
	let header_len = u32::from_le_bytes(word);
	ensure!(
		header_len <= MAX_HEADER_LEN,
		"delta header is too long ({header_len} bytes)"
	);
	let mut header = vec![0; header_len as usize];
	reader
		.read_exact(&mut header)
		.context("Failed to read delta header")?;
	rmp_serde::from_slice(&header).context("Failed to decode delta header")
}

/// Decompresses the rest of a delta after [`read_delta_header`], given the
/// base's decompressed data
pub fn read_delta_body<R>(
	reader: R,
	header: &DeltaHeader,
	base_raw: &[u8],
) -> Result<Vec<u8>>
where
	R: BufRead,
{
	ensure!(
		hash_base(base_raw) == header.base_xxh64,
		"base build {} changed since the delta was written",
		header.base_hash
	);
	let mut dec = zstd::Decoder::with_ref_prefix(reader, base_raw)
		.context("Failed to start zstd stream")?;
	dec.window_log_max(ZSTD_MAX_WINDOW_LOG)?;
	let mut raw = Vec::with_capacity(usize::try_from(header.raw_len)?);
	dec.read_to_end(&mut raw)
		.context("Failed to decompress delta")?;
	ensure!(
		raw.len() as u64 == header.raw_len,
		"delta decompressed to {} bytes, expected {}",
		raw.len(),
		header.raw_len
	);
	Ok(raw)
}

/// Reads a delta file, calling `load_base` with its header to get the
/// base's decompressed data
pub fn read_delta_file<F>(path: &Path, load_base: F) -> Result<Vec<u8>>
where
	F: FnOnce(&DeltaHeader) -> Result<Vec<u8>>,
{
	let f = fs::File::open(path)
		.with_context(|| format!("Failed to open {}", path.display()))?;
	let mut reader = BufReader::new(f);
	let header = read_delta_header(&mut reader)
		.with_context(|| format!("Failed to read {}", path.display()))?;
	let base_raw = load_base(&header)?;
	read_delta_body(reader, &header, &base_raw)
		.with_context(|| format!("Failed to read {}", path.display()))
}

/// Just the header of a delta file
pub fn read_delta_header_file(path: &Path) -> Result<DeltaHeader> {
	let f = fs::File::open(path)
		.with_context(|| format!("Failed to open {}", path.display()))?;
	read_delta_header(&mut BufReader::with_capacity(MAX_HEADER_LEN as usize, f))
		.with_context(|| format!("Failed to read {}", path.display()))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample(seed: u8) -> Vec<u8> {
		(0..200_000u32)
			.flat_map(|i| {
				format!("module {i} = function(){{ return {seed}; }};\n")
					.into_bytes()
			})
			.collect()
	}

	#[test]
	fn round_trip() {
		let base = sample(1);
		let mut target = base.clone();
		target.splice(1000..1010, b"changed!".iter().copied());
		target.extend_from_slice(b"a new module");

		let delta =
			write_delta(Vec::new(), &target, "ab12", &base, 10).unwrap();
		// nearly everything should come from the base
		assert!(delta.len() < target.len() / 100, "delta {}", delta.len());

		let mut reader = delta.as_slice();
		let header = read_delta_header(&mut reader).unwrap();
		assert_eq!(header.base_hash, "ab12");
		assert_eq!(header.raw_len, target.len() as u64);
		let back = read_delta_body(reader, &header, &base).unwrap();
		assert_eq!(back, target);
	}

	#[test]
	fn changed_base_is_an_error() {
		let base = sample(1);
		let target = sample(2);
		let delta = write_delta(Vec::new(), &target, "ab12", &base, 3).unwrap();

		let mut reader = delta.as_slice();
		let header = read_delta_header(&mut reader).unwrap();
		let other_base = sample(3);
		let err = read_delta_body(reader, &header, &other_base).unwrap_err();
		assert!(err.to_string().contains("changed"), "{err}");
	}

	#[test]
	fn rejects_plain_zstd() {
		let plain = zstd::encode_all(sample(1).as_slice(), 3).unwrap();
		assert!(read_delta_header(&mut plain.as_slice()).is_err());
	}
}
