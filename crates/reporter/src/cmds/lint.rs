use anyhow::{Context as _, Result, bail};
use oxc_allocator::Allocator;
use std::{sync::mpsc, time::Instant};
use tracing::{debug, info};
use vencord_ast_parser::{VencordAstParser, diag::LocalSource};

use crate::{util::MultiProgressWrapper, vc};
// TODO: exit success on ctrl-c
pub fn lint(
	mut cli: crate::Cli,
	global_bar: &MultiProgressWrapper,
) -> Result<()> {
	info!("Starting linting");
	let start_time = Instant::now();
	let mut i = 0u32;
	let alloc = Allocator::new();
	if cli.vc_opts.plugin_dirs.is_empty() {
		cli.vc_opts.plugin_dirs =
			vc::infer_plugin_dirs(&cli.vc_opts.vencord_dir)
				.context("Failed to infer plugin dirs")?;
		debug!("Inferred plugin dirs: {:?}", cli.vc_opts.plugin_dirs);
	}
	let mut plugins = Vec::new();
	for plugin_base_dir in cli
		.vc_opts
		.plugin_dirs
		.iter()
		.map(|d| cli.vc_opts.vencord_dir.join(d))
	{
		if !plugin_base_dir.exists() {
			bail!(
				"Plugin base dir {} doesn't exist",
				plugin_base_dir.display()
			);
		}
		vc::glob_plugins_for_dir(&plugin_base_dir, &mut plugins)?;
	}
	let (mut tx, rx) = mpsc::channel();
	for plugin in plugins {
		let path = plugin.entry_point.to_string_lossy();
		let mut parser = match VencordAstParser::try_new(
			&alloc,
			&plugin.entry_source,
			Some(&path),
		) {
			Ok(parser) => parser,
			Err(e) => {
				i += 1;
				global_bar.suspend(|| {
					eprintln!("{e:?}");
				});
				continue;
			}
		};
		parser.diag_ch = Some(tx);
		parser.collect_diagnostics();
		while let Ok(diag) = rx.try_recv() {
			let report = miette::Error::new(diag);
			let e = LocalSource {
				inner: report,
				source: &plugin.entry_source,
				name: &path,
			};
			i += 1;
			global_bar.suspend(|| {
				eprintln!("{e:?}");
			});
		}
		tx = parser.diag_ch.take().unwrap();
	}
	info!(
		"Finished linting plugins in {:.2?}. Found {i} issues.",
		start_time.elapsed()
	);
	Ok(())
}
