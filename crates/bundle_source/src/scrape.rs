//! Scraping builds directly from discord

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context as _, Result, bail};
use discord_scraper::{
	JsScraper,
	ScrapeProgress,
	ScrapedBundle,
	ScrapedModules,
	make_reqwest_client,
	scrape_full_bundle,
	util::ByteStr,
};
use explorer_types::{Channel, ModuleId};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use tracing::{debug, error, info, warn};

use crate::cache;

pub type ScrapedOutput = HashMap<ModuleId, String>;

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedBranch {
	pub channel: Channel,
	pub modules: ScrapedOutput,
	pub build_hash: String,
	/// css chunk file name to its contents
	///
	/// empty in entries cached before css was kept
	#[serde(default)]
	pub css_chunks: HashMap<SmolStr, String>,
}

pub struct IndexResponse {
	pub text: ByteStr,
	pub build_hash: String,
}

/// Fetches the app html for `channel`, along with its build hash
pub async fn fetch_index(
	client: &ClientWithMiddleware,
	channel: Channel,
) -> Result<IndexResponse> {
	let url = channel.app_base();
	let res = client.get(url).send().await?;
	let build_hash = if let Some(build_hash) = res.headers().get("x-build-id") {
		match build_hash.to_str() {
			Ok(s) => {
				info!("{channel:?} build hash: {s}");
				String::from(s)
			}
			Err(e) => {
				bail!("Failed to read build hash from response header: {e:?}");
			}
		}
	} else {
		bail!("Response did not include build hash header");
	};
	let bytes = res.bytes().await?;
	let text = ByteStr::try_from(bytes).context("Invalid response body")?;
	Ok(IndexResponse { text, build_hash })
}

/// Scrapes the raw modules and css of the current build on `channel`, going
/// through the on-disk cache
///
/// Cache entries written before css was kept have no css; `want_css`
/// re-scrapes those instead of returning them.
pub async fn fetch_modules(
	channel: Channel,
	progress: Arc<dyn ScrapeProgress>,
	want_css: bool,
) -> Result<ScrapedBranch> {
	info!("Fetching build from {channel:?} channel");
	progress.set_stage("Fetching index HTML");
	let client =
		make_reqwest_client().context("Failed to create HTTP client")?;
	let IndexResponse { text, build_hash } =
		fetch_index(&client, channel).await?;
	let cache_key = make_cache_key(build_hash.clone());
	progress.set_stage("Reading cache");
	match cache::read::<ScrapedBranch>(&cache_key).await {
		Ok(Some(data)) if want_css && data.css_chunks.is_empty() => {
			info!("Cached build has no css, scraping it again");
		}
		Ok(Some(data)) => {
			info!("Cache hit on fetching build");
			return Ok(data);
		}
		Ok(None) => {
			debug!("Cache miss on fetching build");
		}
		Err(e) => {
			if e.is_deserialize() {
				warn!("Failed to deserialize cache file. invalidating entry");
				if let Err(e) = cache::invalidate(&cache_key).await {
					error!("Failed to invalidate cache entry {e:?}");
				}
			}
			warn!(
				"Failed to read from cache, falling back to scraping via network. {e:?}"
			);
		}
	}
	let ScrapedModules {
		modules,
		css_chunks,
		..
	} = JsScraper::scrape(text.as_ref(), channel, client, progress).await?;
	let output = ScrapedBranch {
		channel,
		modules,
		build_hash,
		css_chunks,
	};
	// we need to await this here so the task isn't dropped on process exit
	match cache::write(&cache_key, &output, None).await {
		Ok(()) => {
			info!("Wrote modules to cache");
		}
		Err(e) => {
			warn!("Failed to write modules to cache. {e:?}");
		}
	}
	Ok(output)
}

/// Scrapes and fully parses the current build on `channel`
///
/// Unlike [`fetch_modules`], this is not cached
pub async fn fetch_full_bundle(
	channel: Channel,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<ScrapedBundle> {
	progress.set_stage("Fetching index HTML");
	let client =
		make_reqwest_client().context("Failed to create HTTP client")?;
	let IndexResponse { text, build_hash } =
		fetch_index(&client, channel).await?;
	scrape_full_bundle(text.as_ref(), channel, build_hash, client, progress)
		.await
}

fn make_cache_key(mut build_hash: String) -> String {
	debug_assert!(!build_hash.is_empty(), "build hash should not be empty");
	const SUFFIX: &str = ".cache";
	if !build_hash.ends_with(SUFFIX) {
		build_hash.push_str(SUFFIX);
	}
	build_hash
}
