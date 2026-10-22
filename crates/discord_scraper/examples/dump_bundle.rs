//! Scrapes the current Discord build and dumps its
//! [`FullBundle`](explorer_types::FullBundle) as JSON to stdout.
//!
//! The output is compact; pipe it through `jq` to pretty-print it.
//!
//! ```sh
//! cargo run -p discord_scraper --example dump_bundle [stable|canary] > bundle.json
//! ```
use std::{
	env,
	io::{self, BufWriter, Write as _},
	sync::Arc,
};

use anyhow::{Context, Result, bail};
use discord_scraper::{
	BarProgress,
	ScrapedBundle,
	make_reqwest_client,
	scrape_full_bundle,
};
use explorer_types::Channel;
use miette_ui::install_miette_hook;
use term_progress::MultiProgressWrapper;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

fn install_tracing(bars: MultiProgressWrapper) {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new("info"));
	fmt()
		.with_env_filter(filter)
		.with_writer(move || bars.writer())
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

#[tokio::main]
async fn main() -> Result<()> {
	let bars = MultiProgressWrapper::default();
	install_tracing(bars.clone());
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

	let ScrapedBundle { bundle, intl: _ } = scrape_full_bundle(
		&html,
		channel,
		build_hash,
		client,
		Arc::new(BarProgress::new(bars, channel)),
	)
	.await?;
	info!(
		"scraped {} modules from build {}",
		bundle.modules.len(),
		bundle.metadata.build_number
	);

	let mut out = BufWriter::new(io::stdout().lock());
	serde_json::to_writer(&mut out, &bundle)?;
	out.flush()?;
	Ok(())
}
