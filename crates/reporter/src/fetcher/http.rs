use anyhow::{Context, Result};
use explorer_types::{FullBundle, TimestampQueryResults};
use reqwest_middleware::reqwest::{self, StatusCode};
use tracing::{debug, info, instrument};

const BASE_URL: &str = "https://s-d-br.sadan.zip";
const PREVIOUS_BUILD_META_URL: &str = "/builds/before/hash";

#[instrument]
pub async fn fetch_previous_build_meta(
	hash: &str,
) -> Result<Option<TimestampQueryResults>> {
	let endpoint = format!("{BASE_URL}{PREVIOUS_BUILD_META_URL}/{hash}");
	debug!("Fetching previous build metadata from {endpoint}");
	let res = reqwest::get(&endpoint)
		.await
		.context("Failed to send request")?;
	if res.status() == StatusCode::NOT_FOUND {
		info!("No previous build found");
		return Ok(None);
	}
	_ = res.error_for_status_ref()?;
	let bts = res
		.bytes()
		.await
		.context("Failed to read response body")?;
	let data = rmp_serde::from_slice(&bts)
		.context("Failed to deserialize response body")?;
	Ok(Some(data))
}

#[instrument]
/// 404 is an error
pub async fn fetch_full_bundle(build_hash: &str) -> Result<FullBundle> {
	let endpoint = format!("{BASE_URL}/build/{build_hash}/full");
	debug!("Fetching full bundle from {endpoint}");
	let res = reqwest::get(&endpoint)
		.await
		.context("Failed to send request")?
		.error_for_status()?;
	let bts = res
		.bytes()
		.await
		.context("Failed to read response body")?;
	let data = tokio::task::spawn_blocking(move || -> Result<_> {
		let raw = zstd::decode_all(&*bts)?;
		let data = rmp_serde::from_slice(&raw)?;
		Ok(data)
	})
	.await
	.context("JoinError")??;
	Ok(data)
}
