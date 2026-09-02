use super::diagnose_patch;
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use explorer_server_core::Channel;
use explorer_types::{BundleMetadata, FullBundle};
use jiff::{Timestamp, tz::TimeZone};
use miette::{Diagnostic, Report, Severity, miette};
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum PreviousBundle {
	Full(FullBundle),
	Scraped(ScrapedBranch),
}

impl Default for PreviousBundle {
	fn default() -> Self {
		Self::Full(FullBundle::default())
	}
}

#[derive(Debug, Default)]
pub struct BuildDiff {
	/// the first broken build after the working build
	pub broken: PreviousBundle,
	/// the last working build right before the broken build
	pub working: FullBundle,
}

impl PreviousBundle {
	pub const fn modules(&self) -> &ScrapedOutput {
		match self {
			Self::Full(FullBundle { modules, .. })
			| Self::Scraped(ScrapedBranch { modules, .. }) => modules,
		}
	}

	pub const fn modules_mut(&mut self) -> &mut ScrapedOutput {
		match self {
			Self::Full(FullBundle { modules, .. })
			| Self::Scraped(ScrapedBranch { modules, .. }) => modules,
		}
	}

	/// the raw bytes of the build hash; use [`encode_build_hash`] for the hex
	/// representation
	pub fn build_hash(&self) -> &[u8] {
		match self {
			Self::Full(FullBundle {
				metadata: Some(BundleMetadata { build_hash, .. }),
				..
			})
			| Self::Scraped(ScrapedBranch { build_hash, .. }) => build_hash,
			_ => &[],
		}
	}

	pub fn build_number(&self) -> Option<u32> {
		match self {
			Self::Full(FullBundle { metadata, .. }) => metadata
				.as_ref()
				.map(|m| m.build_number),
			Self::Scraped(ScrapedBranch { .. }) => None,
		}
	}

	pub fn timestamp(&self) -> Option<jiff::Timestamp> {
		match self {
			Self::Full(full_bundle) => full_bundle
				.metadata
				.as_ref()
				.map(|m| m.first_seen_as_timestamp()),
			Self::Scraped(_) => None,
		}
	}
}

use crate::{
	diag::ReporterError,
	fetcher::{ScrapedBranch, ScrapedOutput, http::fetch_previous_build_meta},
	util::MultiProgressWrapper,
	vc,
};

type R = miette::Result<Option<BuildDiff>>;

async fn do_find_recursive(
	prev_timestamp: Timestamp,
	channel: Channel,
	patch: Arc<Vec<vc::Plugin>>,
	global_bar: &MultiProgressWrapper,
	prev_diag: &ReporterError,
	seen: &mut HashSet<String>,
	prev_bundle: PreviousBundle,
) -> R {
	let prev_build_meta = fetch_previous_build_meta(prev_timestamp)
		.await
		.map_err(Report::msg)?;
	let Some(meta) = prev_build_meta else {
		error!("No previous build found. Cannot fix patch");
		return Ok(None);
	};
	let meta = meta.before.ok_or_else(|| {
		miette!("server did not provide build but returned ok")
	})?;
	let build_hash = meta.build_hash_hex();
	info!("Found previous build with hash {build_hash}");

	if seen.contains(&build_hash) {
		error!(
			"Cycle detected: build hash {build_hash} has already been visited. Cannot continue."
		);
		return Ok(None);
	}

	let strftime = meta
		.first_seen_as_zoned()
		.with_time_zone(TimeZone::system())
		.strftime("%A %B %d %Y %I:%M:%S %p %Z");
	info!("It was built on {strftime}");
	info!("fetching full bundle for build {build_hash}");
	let mut full_bundle = crate::fetcher::http::fetch_full_bundle(&build_hash)
		.await
		.map_err(|e| Report::msg(e).context("Failed to fetch full bundle"))?;
	info!("Attempting to diagnose patch on previous build");
	global_bar.clear();
	let bundle_tmp: Arc<HashMap<u32, String>> = Arc::new(full_bundle.modules);
	let diag =
		diagnose_patch(channel, bundle_tmp.clone(), patch.clone(), global_bar)
			.await;
	full_bundle.modules =
		Arc::into_inner(bundle_tmp).expect("bundle_tmp has outstanding refs");
	match diag {
		None => Ok(Some(BuildDiff {
			working: full_bundle,
			broken: prev_bundle,
		})),
		Some(diag) => {
			let is_same_diag = diag == *prev_diag;
			let new_diag_is_error = diag.severity() == Some(Severity::Error);
			debug_assert_eq!(
				prev_diag.severity(),
				Some(Severity::Error),
				"HANDLE"
			);
			if is_same_diag || new_diag_is_error {
				// we have to recurse
				if !is_same_diag {
					debug_assert!(new_diag_is_error, "logic error");
					warn!("diagnosis ");
				}
				seen.insert(build_hash);
				Box::pin(do_find_recursive(
					meta.first_seen
						.map_or_else(Timestamp::now, Into::into),
					channel,
					patch,
					global_bar,
					&diag,
					seen,
					PreviousBundle::Full(full_bundle),
				))
				.await
			} else {
				warn!(
					"Found a build without an error; however, it has a warning. Using it anyway."
				);
				Ok(Some(BuildDiff {
					working: full_bundle,
					broken: prev_bundle,
				}))
			}
		}
	}
}
pub(super) async fn find_last_build(
	prev_hash: &str,
	channel: Channel,
	patch: Arc<Vec<vc::Plugin>>,
	global_bar: &MultiProgressWrapper,
	prev_err: &ReporterError,
	prev_bundle: PreviousBundle,
) -> R {
	let mut seen = HashSet::from([String::from(prev_hash)]);
	let prev_timestamp = prev_bundle
		.timestamp()
		.unwrap_or_else(Timestamp::now);
	do_find_recursive(
		prev_timestamp,
		channel,
		patch,
		global_bar,
		prev_err,
		&mut seen,
		prev_bundle,
	)
	.await
}
