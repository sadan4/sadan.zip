//! Scrapes the current Discord build and runs
//! [`generate_finds`](webpack_ast_parser::WebpackAstParser::generate_finds) on
//! every module, discarding the result.
//!
//! Modules that fail to parse or panic are logged to stderr. Useful for
//! shaking out crashes in find generation against real code.
//!
//! ```sh
//! cargo run -p discord_scraper --example generate_finds [stable|canary]
//! ```
use std::{
	env,
	io,
	panic::{self, AssertUnwindSafe},
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	thread,
};

use anyhow::{Context, Result, bail};
use discord_scraper::{JsScraper, NoProgress, make_reqwest_client};
use explorer_types::{Channel, ModuleId};
use miette_ui::install_miette_hook;
use oxc_allocator::Allocator;
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

/// Runs `generate_finds` on `src`, returning `false` if it failed to parse or
/// panicked
fn generate_finds(id: ModuleId, src: &str) -> bool {
	let mut src = src.to_owned();
	WebpackAstParser::format_module_header(&mut src, id, false);
	let alloc = Allocator::new();
	let parser = match WebpackAstParser::try_new(&alloc, &src) {
		Ok(parser) => parser,
		Err(e) => {
			warn!(%id, "Failed to parse module: {e}");
			return false;
		}
	};
	let res = panic::catch_unwind(AssertUnwindSafe(|| {
		drop(parser.generate_finds());
	}));
	if res.is_err() {
		warn!(%id, "generate_finds panicked");
	}
	res.is_ok()
}

#[tokio::main]
async fn main() -> Result<()> {
	install_tracing();
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

	info!("generating finds");
	let failed = &AtomicUsize::new(0);
	let threads = thread::available_parallelism().map_or(1, usize::from);
	let entries = scraped
		.modules
		.iter()
		.collect::<Vec<_>>();
	let chunk_size = entries.len().div_ceil(threads).max(1);
	thread::scope(|s| {
		for chunk in entries.chunks(chunk_size) {
			s.spawn(move || {
				for &(&id, src) in chunk {
					if !generate_finds(id, src) {
						failed.fetch_add(1, Ordering::Relaxed);
					}
				}
			});
		}
	});

	let failed = failed.load(Ordering::Relaxed);
	info!("{failed}/{} modules failed", entries.len());
	if failed != 0 {
		bail!("{failed} modules failed");
	}
	Ok(())
}
