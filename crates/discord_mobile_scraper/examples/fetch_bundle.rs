use std::{env, fs};

use anyhow::{Context, Result, bail};
use discord_mobile_scraper::{
	fetch_main_bundle,
	fetch_manifest,
	get_latest_version,
};

#[tokio::main]
async fn main() -> Result<()> {
	let out_path = match env::args().nth(1) {
		Some(p) => p,
		None => bail!("usage: fetch_bundle <output-path>"),
	};

	let version = get_latest_version()
		.await
		.context("Failed to get latest version")?;
	eprintln!("latest version: {version:#?}");

	let manifest = fetch_manifest(version)
		.await
		.context("Failed to fetch manifest")?;
	let commit = &manifest.metadata.commit;
	eprintln!("commit: {commit}");

	let bundle = fetch_main_bundle(commit)
		.await
		.context("Failed to fetch main bundle")?;
	eprintln!("fetched {} bytes", bundle.len());

	fs::write(&out_path, &bundle)
		.with_context(|| format!("Failed to write bundle to {out_path:?}"))?;
	eprintln!("wrote bundle to {out_path}");

	Ok(())
}
