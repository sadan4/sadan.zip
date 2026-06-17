use anyhow::{Context as _, Result};
use clap::Args;
use discord_scraper::{
	ScrapeProgress,
	ScrapedModules,
	make_reqwest_client,
	scrape_modules,
};
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use reqwest_middleware::ClientWithMiddleware;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex, OnceLock},
};
use tokio::task;
use tracing::{info, warn};

use crate::{
	Branch,
	util::{ByteStr, MultiProgressWrapper, Stage},
};

pub type ScrapedOutput = HashMap<ModuleId, String>;

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
	bars: &MultiProgressWrapper,
) -> Result<Vec<ScrapedBranch>> {
	let mut futs: Vec<task::JoinHandle<Result<ScrapedBranch>>> =
		Vec::with_capacity(2);
	for &branch in &opts.branches {
		let ch = branch.into();
		let bars2 = bars.clone();
		futs.push(tokio::spawn(async move {
			let scraped = fetch_for_channel(ch, bars2).await?;
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
	bars: MultiProgressWrapper,
) -> Result<ScrapedOutput> {
	info!("Fetching build from {channel:?} channel");
	let pre_bar =
		Stage::new(format!("[{channel:?}]: Scraping build data: "), None)
			.and_attach(&bars);
	pre_bar.msg("Fetching index HTML");
	let client =
		make_reqwest_client().context("Failed to create HTTP client")?;
	let index_response = fetch_index(&client, channel).await?;
	let progress = Arc::new(ReporterProgress {
		bars,
		channel,
		pre_bar: Mutex::new(Some(pre_bar)),
		chunk_bar: OnceLock::new(),
	});
	let ScrapedModules { modules, .. } =
		scrape_modules(index_response.as_ref(), channel, client, progress)
			.await?;
	Ok(modules)
}

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

struct ReporterProgress {
	bars: MultiProgressWrapper,
	channel: Channel,
	pre_bar: Mutex<Option<Stage>>,
	chunk_bar: OnceLock<Stage>,
}

impl ScrapeProgress for ReporterProgress {
	fn set_stage(&self, msg: &'static str) {
		if let Some(b) = self.pre_bar.lock().unwrap().as_ref() {
			b.msg(msg);
		}
	}

	fn set_chunk_total(&self, total: usize) {
		let _ = self.pre_bar.lock().unwrap().take();
		let bar = Stage::new(
			format!("[{:?}]: Parsing Lazy Chunks: ", self.channel),
			Some(total),
		)
		.and_attach(&self.bars);
		let _ = self.chunk_bar.set(bar);
	}

	fn chunk_finished(&self) {
		if let Some(b) = self.chunk_bar.get() {
			b.step();
		}
	}
}
