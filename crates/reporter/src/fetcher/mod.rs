pub mod http;

use anyhow::{Context as _, Result, bail};
use clap::Args;
use discord_scraper::{
	JsScraper,
	ScrapeProgress,
	ScrapedModules,
	make_reqwest_client,
	util::ByteStr,
};
use explorer_server_core::Channel;
use explorer_types::ModuleId;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex, OnceLock},
};
use tokio::task;
use tracing::{debug, error, info, warn};

use crate::{
	Branch,
	cache,
	util::{MultiProgressWrapper, Stage},
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedBranch {
	pub channel: Channel,
	pub modules: ScrapedOutput,
	pub build_hash: String,
}

pub async fn fetch_build(
	opts: FetchOpts,
	bars: &MultiProgressWrapper,
) -> Result<Vec<ScrapedBranch>> {
	if opts.branches.len() > 2 {
		warn!(?opts.branches, "Fetching more than 2 branches at once ????");
	}
	let mut futs: SmallVec<[task::JoinHandle<Result<ScrapedBranch>>; 2]> =
		SmallVec::with_capacity(opts.branches.len());
	for &branch in &opts.branches {
		let ch = branch.into();
		let bars2 = bars.clone();
		futs.push(tokio::spawn(async move {
			let scraped = fetch_for_channel(ch, bars2).await?;
			Ok(scraped)
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
) -> Result<ScrapedBranch> {
	info!("Fetching build from {channel:?} channel");
	let pre_bar =
		Stage::new(format!("[{channel:?}]: Scraping build data: "), None)
			.and_attach(&bars);
	pre_bar.msg("Fetching index HTML");
	let client =
		make_reqwest_client().context("Failed to create HTTP client")?;
	let IndexResponse { text, build_hash } =
		fetch_index(&client, channel).await?;
	let cache_key = make_cache_key(build_hash.clone());
	pre_bar.msg("Reading cache");
	match cache::read(&cache_key).await {
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
	pre_bar.msg("Fetching index HTML");
	let progress = Arc::new(ReporterProgress {
		bars,
		channel,
		pre_bar: Mutex::new(Some(pre_bar)),
		chunk_bar: OnceLock::new(),
		pre_chunk_count: Mutex::new(0),
	});
	let ScrapedModules { modules, .. } =
		JsScraper::scrape(text.as_ref(), channel, client, progress).await?;
	let output = ScrapedBranch {
		channel,
		modules,
		build_hash,
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

fn make_cache_key(mut build_hash: String) -> String {
	debug_assert!(!build_hash.is_empty(), "build hash should not be empty");
	const SUFFIX: &str = ".cache";
	if !build_hash.ends_with(SUFFIX) {
		build_hash.push_str(SUFFIX);
	}
	build_hash
}

pub struct IndexResponse {
	pub text: ByteStr,
	pub build_hash: String,
}

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

struct ReporterProgress {
	bars: MultiProgressWrapper,
	channel: Channel,
	pre_bar: Mutex<Option<Stage>>,
	chunk_bar: OnceLock<Stage>,
	pre_chunk_count: Mutex<usize>,
}

impl ScrapeProgress for ReporterProgress {
	fn set_stage(&self, msg: &'static str) {
		if let Some(b) = self.pre_bar.lock().unwrap().as_ref() {
			b.msg(msg);
		}
	}

	fn set_chunk_total(&self, mut total: usize) {
		let _ = self.pre_bar.lock().unwrap().take();
		let lock = self.pre_chunk_count.lock().unwrap();
		let extra_total = *lock;
		total += extra_total;
		let bar = Stage::new(
			format!("[{:?}]: Parsing Lazy Chunks: ", self.channel),
			Some(total),
		)
		.and_attach(&self.bars);
		for _ in 0..extra_total {
			bar.step();
		}
		self.chunk_bar.set(bar).expect("race");
	}

	fn chunk_finished(&self) {
		if let Some(b) = self.chunk_bar.get() {
			b.step();
		} else {
			*self.pre_chunk_count.lock().unwrap() += 1;
		}
	}
}
