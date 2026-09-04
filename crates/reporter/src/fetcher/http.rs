use std::io::{self, BufReader};

use anyhow::{Context, Result};
use explorer_types::{FullBundle, TimestampQueryResults};
use reqwest_middleware::reqwest::{self, StatusCode};
use tokio_stream::StreamExt as _;
use tokio_util::io::{StreamReader, SyncIoBridge};
use tracing::{debug, info, instrument, warn};

use crate::cache;

const BUF_SIZE: usize = 1024 * 1024;

const BASE_URL: &str = "https://s-d-br.sadan.zip";
const PREVIOUS_BUILD_META_URL: &str = "/builds/before/time";

#[instrument]
pub async fn fetch_previous_build_meta(
	timestamp: u64,
) -> Result<Option<TimestampQueryResults>> {
	let endpoint = format!("{BASE_URL}{PREVIOUS_BUILD_META_URL}/{timestamp}");
	debug!("Fetching previous build metadata from {endpoint}");
	let res = reqwest::get(&endpoint)
		.await
		.context("Failed to send request")?;
	if res.status() == StatusCode::NOT_FOUND {
		info!("No previous build found");
		return Ok(None);
	}
	_ = res.error_for_status_ref()?;
	let body = SyncIoBridge::new(StreamReader::new(
		res.bytes_stream()
			.map(|r| r.map_err(io::Error::other)),
	));
	let data = tokio::task::spawn_blocking(move || -> Result<_> {
		rmp_serde::from_read(BufReader::with_capacity(BUF_SIZE, body))
			.context("Failed to deserialize response body")
	})
	.await
	.context("JoinError")??;
	Ok(Some(data))
}

#[instrument]
/// 404 is an error
pub async fn fetch_full_bundle(build_hash: &str) -> Result<FullBundle> {
	let cache_key = format!("{build_hash}-full");
	match cache::read::<FullBundle>(&cache_key).await {
		Ok(Some(cached_bundle)) => {
			debug!("Loaded full bundle for {} from cache", build_hash);
			return Ok(cached_bundle);
		}
		Ok(None) => {}
		Err(e) => {
			warn!(
				"Failed to read cached full bundle for {}: {}",
				build_hash, e
			);
			if e.is_deserialize() {
				info!("Invalidating corrupted cache for {}", build_hash);
				let _ = cache::invalidate(&cache_key).await;
			}
		}
	}

	let endpoint = format!("{BASE_URL}/build/{build_hash}/full");
	debug!("Fetching full bundle from {endpoint}");
	let res = reqwest::get(&endpoint)
		.await
		.context("Failed to send request")?
		.error_for_status()?;
	let body = SyncIoBridge::new(StreamReader::new(
		res.bytes_stream()
			.map(|r| r.map_err(io::Error::other)),
	));
	let data = tokio::task::spawn_blocking(move || -> Result<_> {
		let raw = zstd::Decoder::new(body)?;
		let data: FullBundle = rmp_serde::from_read(BufReader::with_capacity(
			BUF_SIZE,
			raw,
		))?;
		Ok(data)
	})
	.await
	.context("JoinError")??;

	if let Err(e) = cache::write(&cache_key, &data, None).await {
		warn!("Failed to cache full bundle for {}: {}", build_hash, e);
	}

	Ok(data)
}
