mod find_last_build;
mod fixer;
mod track_module;

use std::sync::Arc;

use explorer_server_core::Channel;
use itertools::Itertools;
use miette::{Context, Diagnostic, Report, Severity, bail};
use tracing::{debug, error, info, trace, warn};
use vencord_ast_parser::diag::LocalSource;

use crate::{
	Cli,
	cmds::fix::find_last_build::{PreviousBundle, find_last_build},
	diag::ReporterError,
	fetcher::{ScrapedOutput, fetch_build},
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
	let mut scraped_branch = base_branch.into_iter().next().unwrap();
	let modules = Arc::new(scraped_branch.modules);
	debug!("found patch with hash {patch_hash:x}");
	let Some(issue) = diagnose_patch(
		channel.into(),
		modules.clone(),
		plugin.clone(),
		global_bar,
	)
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
	let hash = scraped_branch.build_hash.clone();
	scraped_branch.modules =
		Arc::into_inner(modules).expect("modules has outstanding refs");
	let Some(build_diff) = find_last_build(
		&hash,
		channel.into(),
		plugin.clone(),
		global_bar,
		&issue,
		PreviousBundle::Scraped(scraped_branch),
	)
	.await
	.context("Failed to find last working build")?
	else {
		error!("No working build found");
		return Ok(-1);
	};
	info!(
		build_number=?build_diff.broken.build_number(),
		hash=%build_diff.broken.build_hash(),
		"Found oldest broken build",
	);
	info!(
		build_number=%build_diff.working.metadata.build_number,
		hash=%build_diff.working.metadata.build_hash,
		"Found last working build",
	);
	global_bar.clear();
	let code = fixer::dispatch(
		build_diff,
		plugin,
		issue,
		channel.into(),
		global_bar.clone(),
	)
	.await
	.context("Failed to fix patch")?;
	Ok(code)
}
