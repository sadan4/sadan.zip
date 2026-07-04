use std::sync::Arc;

use explorer_server_core::Channel;
use itertools::Itertools;
use miette::{Context, Diagnostic, Report, Severity, bail};
use tracing::{debug, error, info, trace, warn};
use vencord_ast_parser::diag::LocalSource;

use crate::{
	Cli,
	diag::ReporterError,
	fetcher::{ScrapedBranch, ScrapedOutput, fetch_build},
	reporter::{Msg, report_broken_patches},
	util::{MultiProgressWrapper, Stage},
	vc,
};

async fn diagnose_patch(
	channel: Channel,
	output: Arc<ScrapedOutput>,
	patch: Arc<Vec<vc::Plugin>>,
	global_bar: &MultiProgressWrapper,
) -> Option<ReporterError> {
	let mut rx = report_broken_patches(channel, output, patch);
	let mut msgs = Vec::new();
	let mut status = None;
	while let Some(msg) = rx.recv().await {
		match msg {
			Msg::RequestProgressBar(sender) => {
				sender.send(global_bar.clone()).unwrap();
			}
			Msg::Error(diag) => {
				if diag.is_no_warn() {
					trace!(
						"ignoring no_warn diagnostic while diagnosing patch {diag:?}"
					);
				} else {
					msgs.push(diag);
				}
			}
			Msg::Done(s) => {
				status = Some(s);
				break;
			}
		}
	}
	let status = status
		.expect("report_broken_patches should always send a Done message");
	let mut warnings = msgs
		.extract_if(.., |d| d.severity() != Some(Severity::Error))
		.collect_vec();
	let mut errs = msgs;
	match &*errs {
		[] => {
			warn!(
				"diagnosis completed with no errors, attempting to diagnoise warnings"
			);
			match &*warnings {
				[] => None,
				[_one] => {
					info!(
						"Diagnoised the patch in {status:.2?} with a single warning"
					);
					Some(warnings.swap_remove(0))
				}
				[_, ..] => {
					error!(
						"TODO: support a patch with no errors and more than one warning"
					);
					None
				}
			}
		}
		[_one] => {
			info!("Diagnoised the patch in {status:.2?} with a single error");
			Some(errs.swap_remove(0))
		}
		[_, ..] => {
			error!("TODO: support a patch with more than one error");
			None
		}
	}
}

/// filter the given plugins to only include the one with the given patch hash
///
/// it *does* [re-bind plugins ids](vc::bind_plugin_ids)
fn filter_plugins(
	plugins: &mut Vec<vc::Plugin>,
	hash: u64,
) -> miette::Result<()> {
	let (i, j) = 'done: {
		for (i, pl) in plugins.iter().enumerate() {
			for (j, pa) in pl.patches.iter().enumerate() {
				if pa.content_hash() == hash {
					break 'done (i, j);
				}
			}
		}
		bail!("No plugin found")
	};
	plugins.swap(0, i);
	plugins[0].patches.swap(0, j);
	plugins[0].patches.truncate(1);
	plugins.truncate(1);
	vc::bind_plugin_ids(plugins);
	Ok(())
}

mod find_last_build {
	use super::diagnose_patch;
	use std::{collections::HashSet, sync::Arc};

	use explorer_server_core::Channel;
	use explorer_types::FullBundle;
	use jiff::tz::TimeZone;
	use miette::{Diagnostic, Report, Severity, miette};
	use tracing::{error, info, warn};

	use crate::{diag::ReporterError, util::MultiProgressWrapper, vc};
	type R = miette::Result<Option<FullBundle>>;
	async fn do_find_recursive(
		prev_hash: &str,
		channel: Channel,
		patch: Arc<Vec<vc::Plugin>>,
		global_bar: &MultiProgressWrapper,
		prev_diag: ReporterError,
		seen: &mut HashSet<String>,
	) -> R {
		let prev_build_meta =
			crate::fetcher::http::fetch_previous_build_meta(prev_hash)
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
		let diag = diagnose_patch(
			channel,
			bundle_tmp.clone(),
			patch.clone(),
			global_bar,
		)
		.await;
		full_bundle.modules = Arc::into_inner(bundle_tmp)
			.expect("bundle_tmp has outstanding refs");
		match diag {
			None => Ok(Some(full_bundle)),
			Some(diag) => {
				let is_same_diag = diag == prev_diag;
				let new_diag_is_error =
					diag.severity() == Some(Severity::Error);
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
					seen.insert(meta.build_hash);
					Box::pin(do_find_recursive(
						&full_bundle.metadata.build_hash,
						channel,
						patch,
						global_bar,
						diag,
						seen,
					))
					.await
				} else {
					warn!(
						"Found a build without an error; however, it has a warning. Using it anyway."
					);
					Ok(Some(full_bundle))
				}
			}
		}
	}
	pub(super) async fn find(
		prev_hash: &str,
		channel: Channel,
		patch: Arc<Vec<vc::Plugin>>,
		global_bar: &MultiProgressWrapper,
		prev_err: ReporterError,
	) -> R {
		let mut seen = HashSet::from([prev_hash.to_string()]);
		do_find_recursive(
			prev_hash, channel, patch, global_bar, prev_err, &mut seen,
		)
		.await
	}
}

pub(super) async fn fix(
	cli: Cli,
	global_bar: &MultiProgressWrapper,
	patch_hash: u64,
) -> miette::Result<i8> {
	let vc_bar =
		Stage::new("Collecting Patches: ", None).and_attach(global_bar);
	if cli.fetch_opts.branches.len() > 1 {
		bail!(
			"Cannot fix a patch on multiple branches at once. please give a single branch to work on."
		);
	}
	let base_branch_fut = tokio::spawn({
		let fetch_opts = cli.fetch_opts.clone();
		let global_bar = global_bar.clone();
		async move { fetch_build(fetch_opts, &global_bar).await }
	});
	let channel = cli.fetch_opts.branches[0];
	let plugin = tokio::spawn(async move {
		let mut plugin = vc::collect_patches(cli.vc_opts, vc_bar)
			.await
			.map_err(Report::msg)?;
		filter_plugins(&mut plugin, patch_hash)?;
		Result::<_, Report>::Ok(Arc::new(plugin))
	})
	.await
	.unwrap()?;
	let base_branch = base_branch_fut
		.await
		.unwrap()
		.map_err(Report::msg)?;
	debug_assert_eq!(
		base_branch.len(),
		1,
		"fetch_build should always return a single build"
	);
	let ScrapedBranch {
		channel: _,
		build_hash,
		modules,
	} = base_branch.into_iter().next().unwrap();
	let modules = Arc::new(modules);
	debug!("found patch with hash {patch_hash:x}");
	let Some(issue) =
		diagnose_patch(channel.into(), modules, plugin.clone(), global_bar)
			.await
	else {
		bail!("Failed to diagnose patch")
	};
	let report = Report::new(issue.clone());
	let plugin_ref = &plugin[0];
	let printer = LocalSource {
		inner: report,
		name: &plugin_ref.entry_point.to_string_lossy(),
		source: &plugin_ref.entry_source,
	};
	info!("Diagnosed patch with hash {patch_hash:x} as issue \n{printer:?}");
	info!("Attempting to find last build where the patch still works");
	let Some(last_working_build) = find_last_build::find(
		&build_hash,
		channel.into(),
		plugin.clone(),
		global_bar,
		issue,
	)
	.await
	.context("Failed to find last working build")?
	else {
		error!("No working build found");
		return Ok(-1);
	};
	info!(
		"Found last working build with meta {:#?}",
		&last_working_build.metadata
	);
	Ok(0)
}
