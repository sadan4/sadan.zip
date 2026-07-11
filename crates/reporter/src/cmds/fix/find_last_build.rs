use super::diagnose_patch;
use std::{collections::HashSet, sync::Arc};

use explorer_server_core::Channel;
use explorer_types::{BundleMetadata, FullBundle};
use jiff::tz::TimeZone;
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

	pub fn build_hash(&self) -> &str {
		match self {
			Self::Full(FullBundle {
				metadata: BundleMetadata { build_hash, .. },
				..
			})
			| Self::Scraped(ScrapedBranch { build_hash, .. }) => build_hash,
		}
	}

	pub const fn build_number(&self) -> Option<u32> {
		match self {
			Self::Full(FullBundle {
				metadata: BundleMetadata { build_number, .. },
				..
			}) => Some(*build_number),
			Self::Scraped(ScrapedBranch { .. }) => None,
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
	prev_hash: &str,
	channel: Channel,
	patch: Arc<Vec<vc::Plugin>>,
	global_bar: &MultiProgressWrapper,
	prev_diag: &ReporterError,
	seen: &mut HashSet<String>,
	prev_bundle: PreviousBundle,
) -> R {
	let prev_build_meta = fetch_previous_build_meta(prev_hash)
		.await
		.map_err(|e| Report::msg(e))?;
	let Some(meta) = prev_build_meta else {
		error!("No previous build found. Cannot fix patch");
		return Ok(None);
	};
	let meta = meta.before.ok_or_else(|| {
		miette!("server did not provide build but returned ok")
	})?;
	info!("Found previous build with hash {}", meta.build_hash);

	if seen.contains(&meta.build_hash) {
		error!("Cycle detected: build hash {} has already been visited. Cannot continue.", meta.build_hash);
		return Ok(None);
	}

	let strftime = meta
		.first_seen_as_zoned()
		.with_time_zone(TimeZone::system())
		.strftime("%A %B %d %Y %I:%M:%S %p %Z");
	info!("It was built on {strftime}");
	info!("fetching full bundle for build {}", meta.build_hash);
	let mut full_bundle =
		crate::fetcher::http::fetch_full_bundle(&meta.build_hash)
			.await
			.map_err(|e| {
				Report::msg(e).context("Failed to fetch full bundle")
			})?;
	info!("Attempting to diagnose patch on previous build");
	global_bar.clear();
	let bundle_tmp = Arc::new(full_bundle.modules);
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
				let hash = meta.build_hash.clone();
				seen.insert(meta.build_hash);
				Box::pin(do_find_recursive(
					&hash,
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
	let mut seen = HashSet::from([prev_hash.to_string()]);
	do_find_recursive(
		prev_hash,
		channel,
		patch,
		global_bar,
		prev_err,
		&mut seen,
		prev_bundle,
	)
	.await
}
