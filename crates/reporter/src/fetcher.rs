use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::Args;
use explorer_types::{BuildList, BundleMetadata, FullBundle};
use itertools::Itertools;
use tokio::fs;
use tracing::{debug, instrument};

use crate::util::{Stage, read_struct};

// TODO: add filter CLI option
#[expect(dead_code)]
#[derive(Default, Debug, Clone)]
pub enum BuildFilter {
    #[default]
    Latest,
    Number(u32),
    Hash(String),
}

impl BuildFilter {
    pub fn choose(&self, mut from: impl Iterator<Item = BundleMetadata>) -> Option<BundleMetadata> {
        match self {
            Self::Latest => from.max_by_key(|f| f.first_seen),
            Self::Number(build_number) => from.find(|build| build.build_number == *build_number),
            Self::Hash(build_hash) => from.find(|build| build.build_hash == *build_hash),
        }
    }
}

// const DEFAULT_BACKEND_URL: &str = "https://s-d-br.sadan.zip";
const DEFAULT_BACKEND_URL: &str = "http://localhost:8484";

#[derive(Args, Debug)]
pub struct FetchOpts {
    // /// Try to run reporter against the build with this number. Fails if the build can't be found.
    // build_number: Option<u32>,
    /// The backend URL to fetch builds from.
    /// You should not need to pass this in most cases.
    #[arg(long, default_value = DEFAULT_BACKEND_URL)]
    backend_url: String,
    /// A path to a local copy of a discord bundle.
    ///
    /// This should be in the format of a zstd compressed, msgpack encoded file.
    ///
    /// If this is provided, nothing will be fetched from [`Self::backend_url`]
    #[arg(long)]
    bundle_file: Option<PathBuf>,
}

#[instrument(skip(bar))]
pub async fn fetch_build(opts: FetchOpts, bar: Stage) -> Result<FullBundle> {
    if let Some(bundle_file) = opts.bundle_file {
        debug!("Fetching build from disk at {}", bundle_file.display());
        fetch_build_from_disk(&bundle_file).await
    } else {
        fetch_build_from_server(&opts, &bar).await
    }
}

async fn fetch_build_from_disk(path: &Path) -> Result<FullBundle> {
    let data = fs::read(path)
        .await
        .context("Failed to read bundle from disk")?;
    let ret = read_struct(&*data).context("Failed to parse bundle from disk.")?;
    debug!("Read bundle from disk");
    Ok(ret)
}

async fn fetch_build_from_server(opts: &FetchOpts, bar: &Stage) -> Result<FullBundle> {
    let backend_url = opts.backend_url.as_str();
    let filter = BuildFilter::Latest;
    bar.msg("Fetching available discord builds");

    let raw_builds = reqwest::get(format!("{backend_url}/builds")).await?;
    let raw_builds = raw_builds.bytes().await?;
    let list: BuildList = rmp_serde::from_slice(&raw_builds)?;

    if list.builds.is_empty() {
        bail!("No builds found on the server");
    }

    let build = find_filtered_build(list, &filter)
        .context("Failed to filter builds")?
        .context("Failed to find a build matching the provided filter")?;

    bar.msg("Found target build, fetching full build data");

    let full_build = reqwest::get(format!("{backend_url}/build/{}/full", build.build_hash)).await?;
    let data = full_build.bytes().await?;

    debug!("Fetched full build data from server");

    read_struct(&*data)
}

fn find_filtered_build(list: BuildList, filter: &BuildFilter) -> Result<Option<BundleMetadata>> {
    debug_assert!(!list.builds.is_empty());
    list
        .builds
        .into_iter()
        .map(|build| read_struct(&*build))
        .process_results(|iter| filter.choose(iter))
}
