//! Scrapes the current Discord build and dumps its experiments as JSON to
//! stdout.
//!
//! Experiments that fail to parse are logged to stderr. Modules are
//! pretty-printed before parsing so those diagnostics point at readable code
//! instead of a single minified line, which means the spans in the dumped
//! [`Experiment::loc`](explorer_types::experiments::Experiment::loc)s are
//! relative to the formatted module, not the raw one stored in a
//! [`FullBundle`](explorer_types::FullBundle).
//!
//! ```sh
//! cargo run -p discord_scraper --example dump_experiments [stable|canary] > experiments.json
//! ```
use std::{env, io, sync::Arc, thread};

use anyhow::{Context, Result, bail};
use discord_scraper::{
	JsScraper,
	NoProgress,
	experiments::ParsedBundle,
	make_reqwest_client,
};
use explorer_types::{Channel, ModuleId};
use miette_ui::install_miette_hook;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};
use webpack_ast_parser::WebpackAstParser;

const INDENT: u8 = 4;

fn install_tracing() {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new("info"));
	fmt()
		.with_env_filter(filter)
		.with_writer(io::stderr)
		.with_ansi_sanitization(false)
		.init();
}

fn parse_channel() -> Result<Channel> {
	match env::args().nth(1).as_deref() {
		None | Some("stable") => Ok(Channel::Stable),
		Some("canary") => Ok(Channel::Canary),
		Some(other) => bail!("Unknown channel {other:?}"),
	}
}

/// Adds the module header and pretty-prints `src`, falling back to the
/// unformatted module if formatting fails
fn format_module(id: ModuleId, src: &mut String) {
	WebpackAstParser::format_module_header(src, id, false);
	match pretty_printer::format_to_str(src, INDENT) {
		Ok(formatted) => *src = formatted,
		Err(e) => warn!(%id, "Failed to format module, using it as-is: {e}"),
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	install_tracing();
	// renders the diagnostics of experiments that fail to parse
	install_miette_hook(true);
	let channel = parse_channel()?;

	let client = make_reqwest_client()?;
	let res = client
		.get(channel.app_base())
		.send()
		.await?;
	let build_hash = res
		.headers()
		.get("x-build-id")
		.context("Response did not include build hash header")?
		.to_str()?
		.to_owned();
	info!("{channel:?} build hash: {build_hash}");
	let html = res.text().await?;

	let scraped =
		JsScraper::scrape(&html, channel, client, Arc::new(NoProgress)).await?;
	info!(
		"scraped {} modules from build {}",
		scraped.modules.len(),
		scraped.build_number
	);
	let mut modules = scraped.modules;

	info!("formatting modules");
	let threads = thread::available_parallelism().map_or(1, usize::from);
	let mut entries = modules.iter_mut().collect::<Vec<_>>();
	let chunk_size = entries.len().div_ceil(threads).max(1);
	thread::scope(|s| {
		for chunk in entries.chunks_mut(chunk_size) {
			s.spawn(|| {
				for (id, src) in chunk {
					format_module(**id, src);
				}
			});
		}
	});

	let experiments = ParsedBundle::new(&modules)?
		.find_experiments()
		.await?;
	info!("found {} experiments", experiments.len());
	serde_json::to_writer_pretty(io::stdout().lock(), &experiments)?;
	Ok(())
}
