use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use explorer_server_core::Channel;
use itertools::Itertools;
use miette::{Diagnostic as _, NamedSource, Severity::Warning};
use tokio::{sync::mpsc, time::Instant};
use tracing::{info, warn};

use crate::{
	SourceWrapper,
	fetcher::{ScrapedBranch, ScrapedOutput, fetch_build},
	reporter::{Msg, report_broken_patches},
	util::{MultiProgressWrapper, Stage, join_all},
	vc::{Plugin, collect_patches},
};

struct ChannelStatus {
	errors: Vec<String>,
	modules: Arc<ScrapedOutput>,
	start: Instant,
	channel: Channel,
	plugins: Arc<Vec<Plugin>>,
	rx: mpsc::Receiver<Msg>,
	bars: MultiProgressWrapper,
	/// [`crate::Cli::no_warnings`]
	no_warnings: bool,
	/// [`crate::Cli::dump_on_error`]
	dump_on_error: bool,
	end: tokio::sync::OnceCell<Duration>,
}

impl ChannelStatus {
	fn new(
		channel: Channel,
		build_data: Arc<ScrapedOutput>,
		plugins: Arc<Vec<Plugin>>,
		bars: MultiProgressWrapper,
		no_warnings: bool,
		dump_on_error: bool,
	) -> Self {
		let start = Instant::now();
		let rx =
			report_broken_patches(channel, build_data.clone(), plugins.clone());
		Self {
			errors: Vec::new(),
			modules: build_data,
			start,
			channel,
			plugins,
			rx,
			bars,
			no_warnings,
			dump_on_error,
			end: tokio::sync::OnceCell::new(),
		}
	}
	async fn run(&mut self) {
		while let Some(msg) = self.rx.recv().await {
			match msg {
				Msg::RequestProgressBar(tx) => {
					tx.send(self.bars.clone()).unwrap();
				}
				Msg::Done(raw_time) => {
					if self.end.set(raw_time).is_err() {
						warn!("Msg::Done sent more than once");
					}
					break;
				}
				Msg::Error(e) => 'm: {
					let e = if (e.severity() == Some(Warning)
						&& self.no_warnings)
						|| e.is_no_warn()
					{
						break 'm;
					} else {
						e
					};
					if self.dump_on_error
						&& let Some(m_id) = e.module_id()
					{
						if self.modules.contains_key(&m_id) {
							let target_build = self.modules.clone();
							let channel = self.channel;
							tokio::spawn(async move {
								let path = format!(
									".reporter-modules/{channel:?}/{m_id}.js"
								);
								let module = target_build.get(&m_id).unwrap();
								tokio::fs::write(path, module).await
							});
						} else {
							warn!(
								"expected target_build to have the contents of module {m_id}"
							);
						}
					}
					let id = e.plugin_id();
					let path = &self.plugins[id as usize].entry_point;
					let source = SourceWrapper(self.plugins.clone(), id);
					let report = miette::Error::new(e).with_source_code(
						NamedSource::new(path.to_string_lossy(), source)
							.with_language("JavaScript"),
					);
					self.errors.push(format!("{report:?}"));
				}
			}
		}
	}
	fn finish(&mut self) {
		let channel = self.channel;
		let Some(raw_time) = self.end.get() else {
			warn!(?channel, "Msg::Done never called");
			return;
		};
		let num_errs = self.errors.len();
		self.bars.suspend(|| {
			for error in self.errors.drain(..) {
				eprintln!("{error}");
			}
		});
		let time = self.start.elapsed();
		info!(
			"Finished report for {channel:?} in {time:.2?} (raw time: {raw_time:.2?}). {num_errs} error(s) reported."
		);
	}
}

pub(super) struct FullReporterResult {
	pub plugins: Arc<Vec<Plugin>>,
	pub modules: Vec<(Channel, Arc<ScrapedOutput>)>,
	pub num_errs: usize,
}

pub async fn run_reporter(
	cli: &crate::Cli,
	bars: &MultiProgressWrapper,
) -> anyhow::Result<FullReporterResult> {
	let patches_bar = Stage::new("Collecting Patches: ", None).and_attach(bars);
	// FIXME: don't wrap in spawn
	let vc_opts = cli.vc_opts.clone();
	let patches_fut =
		tokio::spawn(
			async move { collect_patches(vc_opts, patches_bar).await },
		);
	let fetch_opts = cli.fetch_opts.clone();
	let bars2 = bars.clone();
	let target_build_fut =
		tokio::spawn(async move { fetch_build(fetch_opts, &bars2).await });
	let (plugins, target_build) = tokio::join!(patches_fut, target_build_fut);
	let plugins = Arc::new(plugins??);
	let scraped_outputs = target_build??
		.into_iter()
		.map(|ScrapedBranch { channel, out }| (channel, Arc::new(out)))
		.collect_vec();
	run_with_data(scraped_outputs, plugins, cli, bars).await
}

pub(super) async fn run_with_data(
	scraped_outputs: Vec<(Channel, Arc<ScrapedOutput>)>,
	plugins: Arc<Vec<Plugin>>,
	cli: &crate::Cli,
	bars: &MultiProgressWrapper,
) -> Result<FullReporterResult> {
	let mut pending_checks = scraped_outputs
		.into_iter()
		.map(|(channel, modules)| {
			ChannelStatus::new(
				channel,
				modules,
				plugins.clone(),
				bars.clone(),
				cli.no_warnings,
				cli.dump_on_error,
			)
		})
		.collect_vec();
	// if we only have one build, we can stream it's output
	if pending_checks.len() == 1 {
		let first = pending_checks.pop().unwrap();
		return stream_single_build(first).await;
	}
	// collect results from each build in parallel
	let mut futs = Vec::with_capacity(pending_checks.len());
	for mut check in pending_checks {
		futs.push(tokio::spawn(async move {
			check.run().await;
			check
		}));
	}
	let finished_checks = join_all(futs).await;
	let mut num_errs = 0;
	info!("Printing results");
	let mut out_modules = Vec::with_capacity(finished_checks.len());
	for check in finished_checks {
		let mut check = check.context("Reporter Check")?;
		num_errs += check.errors.len();
		check.finish();
		out_modules.push((check.channel, check.modules));
	}
	Ok(FullReporterResult {
		plugins,
		num_errs,
		modules: out_modules,
	})
}

async fn stream_single_build(
	ChannelStatus {
		errors: _,
		modules,
		start,
		channel,
		plugins,
		mut rx,
		bars,
		no_warnings,
		dump_on_error,
		end: _,
	}: ChannelStatus,
) -> Result<FullReporterResult> {
	let mut num_errs = 0;
	while let Some(msg) = rx.recv().await {
		match msg {
			Msg::RequestProgressBar(tx) => {
				tx.send(bars.clone()).unwrap();
			}
			Msg::Done(raw_time) => {
				info!(
					"Reporter finished in {:.2?}. (raw time: {raw_time:.2?})",
					start.elapsed()
				);
				break;
			}
			Msg::Error(e) => 'm: {
				let e = if (e.severity() == Some(Warning) && no_warnings)
					|| e.is_no_warn()
				{
					break 'm;
				} else {
					e
				};
				num_errs += 1;
				if dump_on_error && let Some(m_id) = e.module_id() {
					if modules.contains_key(&m_id) {
						let target_build = modules.clone();
						tokio::spawn(async move {
							let path = format!("{m_id}.js");
							let module = target_build.get(&m_id).unwrap();
							tokio::fs::write(path, module).await
						});
					} else {
						warn!(
							"expected target_build to have the contents of module {m_id}"
						);
					}
				}
				let id = e.plugin_id();
				let path = &plugins[id as usize].entry_point;
				let source = SourceWrapper(plugins.clone(), id);
				let report = miette::Error::new(e).with_source_code(
					NamedSource::new(path.to_string_lossy(), source)
						.with_language("JavaScript"),
				);
				bars.suspend(|| {
					eprintln!("{report:?}");
				});
			}
		}
	}
	Ok(FullReporterResult {
		plugins,
		num_errs,
		modules: vec![(channel, modules)],
	})
}
