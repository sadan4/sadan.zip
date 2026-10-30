pub mod http;

use std::sync::Arc;

use anyhow::{Context as _, Result};
pub use bundle_source::scrape::{ScrapedBranch, ScrapedOutput};
use clap::Args;
use discord_scraper::BarProgress;
use smallvec::SmallVec;
use tokio::task;
use tracing::warn;

use crate::{Branch, util::MultiProgressWrapper};

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
		let progress = Arc::new(BarProgress::new(bars.clone(), ch));
		futs.push(tokio::spawn(bundle_source::scrape::fetch_modules(
			ch, progress, false,
		)));
	}
	let mut results = Vec::with_capacity(futs.len());
	for fut in futs {
		results.push(fut.await.context("Join error")??);
	}
	Ok(results)
}
