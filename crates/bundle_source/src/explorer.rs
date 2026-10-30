//! Client for the explorer server (`explorer_server`)

use std::io::{self, BufReader, Read};

use anyhow::{Context, Result};
use explorer_types::{
	BuildList,
	BundleMetadata,
	FullBundle,
	TimestampQueryResults,
	intl::IntlMessages,
};
use reqwest_middleware::reqwest::{self, Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio_stream::StreamExt as _;
use tokio_util::io::{StreamReader, SyncIoBridge};
use tracing::{debug, info, instrument, warn};

use crate::cache;

const BUF_SIZE: usize = 1024 * 1024;

pub const DEFAULT_BASE_URL: &str = "https://s-d-br.sadan.zip";

/// A page of [`ExplorerClient::list_builds`]
#[derive(Debug)]
pub struct BuildPage {
	/// newest first
	pub builds: Vec<BundleMetadata>,
	/// pass as `before` to get the next page, `None` on the last page
	pub next_cursor: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ExplorerClient {
	base_url: String,
	client: reqwest::Client,
}

impl Default for ExplorerClient {
	fn default() -> Self {
		Self::new(DEFAULT_BASE_URL)
	}
}

impl ExplorerClient {
	pub fn new(base_url: impl Into<String>) -> Self {
		let mut base_url = base_url.into();
		while base_url.ends_with('/') {
			base_url.pop();
		}
		Self {
			base_url,
			client: reqwest::Client::new(),
		}
	}

	pub fn base_url(&self) -> &str {
		&self.base_url
	}

	async fn get(&self, path: &str) -> Result<Response> {
		let endpoint = format!("{}{path}", self.base_url);
		debug!("GET {endpoint}");
		self.client
			.get(&endpoint)
			.send()
			.await
			.with_context(|| format!("Failed to send request to {endpoint}"))
	}

	/// `None` on 404
	async fn get_opt(&self, path: &str) -> Result<Option<Response>> {
		let res = self.get(path).await?;
		if res.status() == StatusCode::NOT_FOUND {
			return Ok(None);
		}
		Ok(Some(res.error_for_status()?))
	}

	#[instrument(skip(self))]
	pub async fn fetch_previous_build_meta(
		&self,
		timestamp: u64,
	) -> Result<Option<TimestampQueryResults>> {
		let Some(res) = self
			.get_opt(&format!("/builds/before/time/{timestamp}"))
			.await?
		else {
			info!("No previous build found");
			return Ok(None);
		};
		Ok(Some(decode_body(res, false).await?))
	}

	/// `None` if the server has no builds
	pub async fn fetch_latest_build_meta(
		&self,
	) -> Result<Option<BundleMetadata>> {
		let Some(res) = self
			.get_opt("/builds/latest/meta")
			.await?
		else {
			return Ok(None);
		};
		Ok(Some(decode_body(res, false).await?))
	}

	/// `None` if the server does not have the build
	pub async fn fetch_build_meta(
		&self,
		build_hash: &str,
	) -> Result<Option<BundleMetadata>> {
		let Some(res) = self
			.get_opt(&format!("/build/{build_hash}/metadata"))
			.await?
		else {
			return Ok(None);
		};
		Ok(Some(decode_body(res, true).await?))
	}

	/// Lists builds newest first
	///
	/// `before` is a unix timestamp in ms, see [`BuildPage::next_cursor`]
	pub async fn list_builds(
		&self,
		before: Option<u64>,
		limit: Option<usize>,
	) -> Result<BuildPage> {
		let mut query = Vec::with_capacity(2);
		if let Some(before) = before {
			query.push(format!("before={before}"));
		}
		if let Some(limit) = limit {
			query.push(format!("limit={limit}"));
		}
		let mut path = String::from("/builds");
		if !query.is_empty() {
			path.push('?');
			path.push_str(&query.join("&"));
		}
		let res = self
			.get(&path)
			.await?
			.error_for_status()?;
		let list: BuildList = decode_body(res, false).await?;
		let builds = list
			.builds
			.iter()
			.map(|raw| {
				let raw = zstd::Decoder::new(&raw[..])?;
				rmp_serde::from_read(raw)
					.context("Failed to decode build metadata")
			})
			.collect::<Result<_>>()?;
		Ok(BuildPage {
			builds,
			next_cursor: list.next_cursor,
		})
	}

	/// Fetches a full bundle, going through the on-disk cache
	///
	/// 404 is an error
	#[instrument(skip(self))]
	pub async fn fetch_full_bundle(
		&self,
		build_hash: &str,
	) -> Result<FullBundle> {
		self.cached(&format!("{build_hash}-full"), || async {
			let res = self
				.get(&format!("/build/{build_hash}/full"))
				.await?
				.error_for_status()?;
			decode_body(res, true).await
		})
		.await
	}

	/// Fetches the english intl messages of a build, going through the
	/// on-disk cache
	///
	/// `None` if the server has no intl messages for this build
	#[instrument(skip(self))]
	pub async fn fetch_intl(
		&self,
		build_hash: &str,
	) -> Result<Option<IntlMessages>> {
		let key = format!("{build_hash}-intl");
		if let Some(cached) = read_cache(&key).await {
			return Ok(Some(cached));
		}
		let Some(res) = self
			.get_opt(&format!("/build/{build_hash}/intl"))
			.await?
		else {
			return Ok(None);
		};
		let data: IntlMessages = decode_body(res, true).await?;
		write_cache(&key, &data).await;
		Ok(Some(data))
	}

	async fn cached<T, F, Fut>(&self, key: &str, fetch: F) -> Result<T>
	where
		T: DeserializeOwned + serde::Serialize + Send + Sync + 'static,
		F: FnOnce() -> Fut,
		Fut: Future<Output = Result<T>>,
	{
		if let Some(cached) = read_cache(key).await {
			return Ok(cached);
		}
		let data = fetch().await?;
		write_cache(key, &data).await;
		Ok(data)
	}
}

async fn read_cache<T>(key: &str) -> Option<T>
where
	T: DeserializeOwned + Send + 'static,
{
	match cache::read::<T>(key).await {
		Ok(Some(cached)) => {
			debug!("Loaded {key} from cache");
			Some(cached)
		}
		Ok(None) => None,
		Err(e) => {
			warn!("Failed to read cached {key}: {e}");
			if e.is_deserialize() {
				info!("Invalidating corrupted cache for {key}");
				_ = cache::invalidate(key).await;
			}
			None
		}
	}
}

async fn write_cache<T>(key: &str, data: &T)
where
	T: serde::Serialize + Sync,
{
	if let Err(e) = cache::write(key, data, None).await {
		warn!("Failed to cache {key}: {e}");
	}
}

/// Streams a msgpack body, optionally zstd compressed, into `T`
async fn decode_body<T>(res: Response, zstd: bool) -> Result<T>
where
	T: DeserializeOwned + Send + 'static,
{
	let body = SyncIoBridge::new(StreamReader::new(
		res.bytes_stream()
			.map(|r| r.map_err(io::Error::other)),
	));
	tokio::task::spawn_blocking(move || -> Result<T> {
		let reader: Box<dyn Read> = if zstd {
			Box::new(zstd::Decoder::new(body)?)
		} else {
			Box::new(body)
		};
		rmp_serde::from_read(BufReader::with_capacity(BUF_SIZE, reader))
			.context("Failed to deserialize response body")
	})
	.await
	.context("JoinError")?
}
