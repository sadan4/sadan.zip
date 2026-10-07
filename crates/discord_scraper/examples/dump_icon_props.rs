//! Scrapes the current Discord build and lists every module matching
//! [`default_icon_props_export`](webpack_ast_parser::WebpackAstParser::default_icon_props_export),
//! printing `<module id> <export name>` per line to stdout.
//!
//! Modules that fail to parse are logged to stderr.
//!
//! ```sh
//! cargo run -p discord_scraper --example dump_icon_props [stable|canary]
//! ```
use std::{env, io, sync::Arc, thread};

use anyhow::{Context, Result, bail};
use discord_scraper::{JsScraper, NoProgress, make_reqwest_client};
use explorer_types::{Channel, ModuleId};
use oxc_allocator::Allocator;
use smol_str::SmolStr;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};
use webpack_ast_parser::WebpackAstParser;

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

/// Returns the default icon props export of `src`, if it has one
fn icon_props_export(id: ModuleId, src: &str) -> Option<SmolStr> {
	let mut src = src.to_owned();
	WebpackAstParser::format_module_header(&mut src, id, false);
	let alloc = Allocator::new();
	let parser = WebpackAstParser::try_new(&alloc, &src)
		.inspect_err(|e| warn!(%id, "Failed to parse module: {e}"))
		.ok()?;
	parser.default_icon_props_export()
}

#[tokio::main]
async fn main() -> Result<()> {
	install_tracing();
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

	info!("searching for icon props modules");
	let threads = thread::available_parallelism().map_or(1, usize::from);
	let entries = scraped
		.modules
		.iter()
		.collect::<Vec<_>>();
	let chunk_size = entries.len().div_ceil(threads).max(1);
	let mut found = thread::scope(|s| {
		entries
			.chunks(chunk_size)
			.map(|chunk| {
				s.spawn(move || {
					chunk
						.iter()
						.filter_map(|&(&id, src)| {
							Some((id, icon_props_export(id, src)?))
						})
						.collect::<Vec<_>>()
				})
			})
			.collect::<Vec<_>>()
			.into_iter()
			.flat_map(|h| h.join().unwrap())
			.collect::<Vec<_>>()
	});
	found.sort_unstable_by_key(|&(id, _)| id);

	info!("found {} icon props modules", found.len());
	for (id, export) in found {
		println!("{id} {export}");
	}
	Ok(())
}
