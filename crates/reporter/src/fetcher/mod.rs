mod scraper;

use anyhow::{Context as _, Result};
use clap::Args;
use explorer_server_core::Channel;
use reqwest_middleware::ClientWithMiddleware;
use tokio::task;
use tracing::{info, warn};

use crate::{
	Branch,
	fetcher::scraper::make_reqwest_client,
	util::{ByteStr, MultiProgressWrapper, Stage},
};

pub use scraper::ScrapedOutput;

#[derive(Args, Debug, Clone)]
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
	/// The branches to run the reporter for
	///
	/// Can be passed more than once
	#[arg(short, long, value_enum, default_values_t = vec![Branch::Stable])]
	pub branches: Vec<Branch>,
}

pub struct ScrapedBranch {
	pub channel: Channel,
	pub out: ScrapedOutput,
}

pub async fn fetch_build(
	opts: FetchOpts,
	bars: &'static MultiProgressWrapper,
) -> Result<Vec<ScrapedBranch>> {
	let mut futs: Vec<task::JoinHandle<Result<ScrapedBranch>>> =
		Vec::with_capacity(2);
	for &branch in &opts.branches {
		let ch = branch.into();
		futs.push(tokio::spawn(async move {
			let scraped = fetch_for_channel(ch, bars).await?;
			Ok(ScrapedBranch {
				channel: ch,
				out: scraped,
			})
		}));
	}
	let mut results = Vec::with_capacity(futs.len());
	for fut in futs {
		results.push(fut.await.context("Join error")??);
	}
	Ok(results)
}

async fn fetch_for_channel(
	channel: Channel,
	bars: &MultiProgressWrapper,
) -> Result<ScrapedOutput> {
	info!("Fetching build from {channel:?} channel");
	let bar = Stage::new(format!("[{channel:?}]: Scraping build data: "), None)
		.and_attach(bars);
	bar.msg("Fetching index HTML");
	let client =
		make_reqwest_client().context("Failed to create HTTP client")?;
	let index_response = fetch_index(&client, channel).await?;
	let scraped = scraper::scrape_build(
		index_response.as_ref(),
		channel,
		bar,
		bars,
		client,
	)
	.await?;
	Ok(scraped)
}

#[allow(clippy::cognitive_complexity, reason = "clippy bug: macros count")]
async fn fetch_index(
	client: &ClientWithMiddleware,
	channel: Channel,
) -> Result<ByteStr> {
	let url = channel.app_base();
	let res = client.get(url).send().await?;
	if let Some(build_hash) = res.headers().get("x-build-id") {
		match build_hash.to_str() {
			Ok(s) => info!("{channel:?} build hash: {s}"),
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
