use std::sync::LazyLock;

use anyhow::Result;
use bundle_source::explorer::ExplorerClient;
use explorer_types::{FullBundle, TimestampQueryResults};

static CLIENT: LazyLock<ExplorerClient> =
	LazyLock::new(ExplorerClient::default);

pub async fn fetch_previous_build_meta(
	timestamp: u64,
) -> Result<Option<TimestampQueryResults>> {
	CLIENT
		.fetch_previous_build_meta(timestamp)
		.await
}

/// 404 is an error
pub async fn fetch_full_bundle(build_hash: &str) -> Result<FullBundle> {
	CLIENT
		.fetch_full_bundle(build_hash)
		.await
}
