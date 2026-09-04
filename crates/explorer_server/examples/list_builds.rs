//! Fetch the `/builds` endpoint and print the debug repr of every build's
//! metadata.

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use explorer_types::{BuildList, BundleMetadata};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	/// base url of the explorer server
	#[arg(long, default_value_t = String::from("http://localhost:8484"))]
	base_url: String,
	/// print the raw [`BuildList`] instead of the decoded metadata
	#[arg(long, default_value_t = false)]
	raw: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	let endpoint = format!("{}/builds", cli.base_url.trim_end_matches('/'));
	let res = reqwest::get(&endpoint)
		.await
		.with_context(|| format!("Failed to GET {endpoint}"))?;
	let status = res.status();
	let body = res
		.bytes()
		.await
		.context("Failed to read response body")?;
	if !status.is_success() {
		bail!(
			"{endpoint} returned {status}: {}",
			String::from_utf8_lossy(&body)
		);
	}

	let build_list: BuildList = rmp_serde::from_slice(&body)
		.context("Failed to deserialize BuildList")?;

	if cli.raw {
		println!("{build_list:#?}");
		return Ok(());
	}

	println!("{} build(s)", build_list.builds.len());
	for (i, build) in build_list.builds.iter().enumerate() {
		let meta_mpk = zstd::Decoder::new(&**build)
			.with_context(|| format!("Failed to decompress build {i}"))?;
		let meta: BundleMetadata = rmp_serde::from_read(meta_mpk)
			.with_context(|| {
				format!("Failed to deserialize metadata for build {i}")
			})?;
		println!("{meta:#?}");
	}

	Ok(())
}
