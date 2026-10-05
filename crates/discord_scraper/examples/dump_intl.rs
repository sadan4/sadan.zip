//! Scrapes the current Discord build and dumps every english intl module
//! found by
//! [`collect_intl_modules`](webpack_ast_parser::WebpackAstParser::collect_intl_modules)
//! into a directory, one pretty-printed `<id>.js` file per module.
//!
//! ```sh
//! cargo run -p discord_scraper --example dump_intl [stable|canary] [out_dir]
//! ```
//!
//! `out_dir` defaults to `intl`.
use std::{env, fs, io, path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail};
use discord_scraper::{
	JsScraper,
	NoProgress,
	experiments::ParsedBundle,
	make_reqwest_client,
};
use explorer_types::Channel;
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

fn parse_channel(arg: Option<&str>) -> Result<Channel> {
	match arg {
		None | Some("stable") => Ok(Channel::Stable),
		Some("canary") => Ok(Channel::Canary),
		Some(other) => bail!("Unknown channel {other:?}"),
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	install_tracing();
	install_miette_hook(true);
	let mut args = env::args().skip(1);
	let channel = parse_channel(args.next().as_deref())?;
	let out_dir = PathBuf::from(
		args.next()
			.unwrap_or_else(|| "intl".into()),
	);

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
	let modules = scraped.modules;

	let bundle = ParsedBundle::new(&modules)?;
	let ids = bundle.find_intl_modules().await?;
	info!("found {} intl modules", ids.len());

	fs::create_dir_all(&out_dir)
		.with_context(|| format!("Failed to create {}", out_dir.display()))?;
	let mut merged_json = serde_json::Map::new();
	for id in ids {
		let Some(src) = modules.get(&id) else {
			warn!(%id, "Intl module not found in bundle");
			continue;
		};
		let mut src = src.clone();
		WebpackAstParser::format_module_header(&mut src, id, false);
		let src =
			pretty_printer::format_to_str(&src, INDENT).unwrap_or_else(|e| {
				warn!(%id, "Failed to format module, using it as-is: {e}");
				src
			});
		let path = out_dir.join(format!("{id}.js"));
		fs::write(&path, src)
			.with_context(|| format!("Failed to write {}", path.display()))?;
		{
			let parser = bundle.get_parser(id).unwrap();
			let intl_json = parser
				.parser()
				.as_json_module()
				.unwrap();
			let path = out_dir.join(format!("{id}.json"));
			let guh = serde_json::from_str::<serde_json::Map<_, _>>(intl_json)
				.unwrap();
			merged_json.extend(guh);
			fs::write(&path, intl_json).with_context(|| {
				format!("Failed to write {}", path.display())
			})?;
		};
		let merged_path = {
			let mut guh = out_dir.clone();
			guh.pop();
			guh.push("merged.json");
			guh
		};
		fs::write(
			&merged_path,
			serde_json::to_string_pretty(&merged_json).unwrap(),
		)?;
	}
	info!("wrote intl modules to {}", out_dir.display());
	Ok(())
}
