use anyhow::{Context as _, Result, bail};
use explorer_types::{BuildList, BundleMetadata, FullBundle};
use itertools::Itertools;
use tracing::{info, instrument};

use crate::util::read_struct;

#[derive(Default, Debug, Copy, Clone)]
pub enum BuildFilter {
    #[default]
    Latest,
    Number(u32),
}

#[instrument]
pub async fn fetch_build(backend_url: &str, filter: BuildFilter) -> Result<FullBundle> {
    info!("Fetching available discord builds");
    let raw_builds = reqwest::get(format!("{backend_url}/builds")).await?;
    let raw_builds = raw_builds.bytes().await?;
    let list: BuildList = rmp_serde::from_slice(&raw_builds)?;

    if list.builds.is_empty() {
        bail!("No builds found on the server");
    }

    let build = find_filtered_build(list, filter)
        .context("Failed to filter builds")?
        .context("Failed to find a build matching the provided filter")?;

    info!("Found target build with hash {}", build.build_hash);
    info!("Fetching full data for target build");

    let full_build = reqwest::get(format!("{backend_url}/build/{}/full", build.build_hash)).await?;
    let data = full_build.bytes().await?;
    read_struct(&*data)
}

fn find_filtered_build(list: BuildList, filter: BuildFilter) -> Result<Option<BundleMetadata>> {
    match filter {
        BuildFilter::Latest => {
            debug_assert!(!list.builds.is_empty());
            Ok(Some(
                list.builds
                    .into_iter()
                    .map(|build| read_struct::<BundleMetadata>(&*build))
                    .process_results(|iter| iter.max_by_key(|f| f.first_seen))?
                    // this unwrap is unreachable because we check if we contain nothing before we call this function
                    .unwrap(),
            ))
        }
        BuildFilter::Number(build_number) => list
            .builds
            .into_iter()
            .map(|build| read_struct::<BundleMetadata>(&*build))
            .process_results(|mut iter| iter.find(|build| build.build_number == build_number)),
    }
}
