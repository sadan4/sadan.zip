mod scraper;
use std::{mem, path::{Path, PathBuf}};

use anyhow::{Context as _, Result, bail};
use clap::Args;
use explorer_server_core::Channel;
use explorer_types::{BuildList, BundleMetadata, FullBundle};
use indicatif::MultiProgress;
use itertools::Itertools;
use reqwest_middleware::ClientWithMiddleware;
use tokio::fs;
use tracing::{debug, info, instrument, warn};

use crate::{
	fetcher::scraper::make_reqwest_client,
	util::{ByteStr, Stage, read_struct},
};

pub use scraper::ScrapedOutput;

#[derive(Args, Debug)]
pub struct FetchOpts {
	// // /// Try to run reporter against the build with this number. Fails if the build can't be found.
	// // build_number: Option<u32>,
	// /// The backend URL to fetch builds from.
	// /// You should not need to pass this in most cases.
	// #[arg(long, default_value = DEFAULT_BACKEND_URL)]
	// backend_url: String,
	// /// A path to a local copy of a discord bundle.
	// ///
	// /// This should be in the format of a zstd compressed, msgpack encoded file.
	// ///
	// /// If this is provided, nothing will be fetched from [`Self::backend_url`]
	// #[arg(long)]
	// bundle_file: Option<PathBuf>,
	/// Fetch the canary build instead of the stable build.
	#[arg(short, long, default_value_t = false)]
	canary: bool,
}

pub async fn fetch_build(
	opts: FetchOpts,
	bars: MultiProgress,
) -> Result<ScrapedOutput> {
	let channel = if opts.canary {
		Channel::Canary
	} else {
		Channel::Stable
	};
	info!("Fetching build from {channel:?} channel");
	let bar = Stage::new("Scraping build data: ", None).and_attach(&bars);
	mem::forget(bar.clone());
	bar.msg("Fetching index HTML");
	let client =
		make_reqwest_client().context("Failed to create HTTP client")?;
	let index_response = fetch_index(&client, channel).await?;
	let scraped =
		scraper::scrape_build(index_response.as_ref(), channel, bar, bars, client)
			.await?;
	Ok(scraped)
}

async fn fetch_index(
	client: &ClientWithMiddleware,
	channel: Channel,
) -> Result<ByteStr> {
	let url = channel.app_base();
	let res = client.get(url).send().await?;
	if let Some(build_hash) = res.headers().get("x-build-id") {
		match build_hash.to_str() {
			Ok(s) => info!("Target build hash: {s}"),
			Err(e) => {
				warn!("Failed to read build hash from response header: {e:?}");
			}
		}
	} else {
		warn!("Response did not include build hash header");
	}
	let bytes = res.bytes().await?;
	let b_str = ByteStr::try_from(bytes).context("Invalid response body")?;
	Ok(b_str)
}
