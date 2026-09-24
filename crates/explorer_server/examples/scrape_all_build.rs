use std::{path::PathBuf, sync::Arc};

use anyhow::{Context as _, bail};
use clap::Parser;
use discord_scraper::make_reqwest_client_with_ua;
use explorer_server_core::{DATA_FILE_NAME, METADATA_FILE_NAME};
use explorer_types::BuildList;
use reqwest::Url;
use tokio::{fs, task::JoinSet};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	/// base url of the explorer server
	#[arg(long, default_value_t = String::from("https://s-d-br.sadan.zip"))]
	base_url: String,
	#[arg(long)]
	target_dir: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();
	let target_dir = Arc::new(PathBuf::from(cli.target_dir));
	std::fs::create_dir_all(&*target_dir)
		.context("Failed to create target directory")?;
	let client = make_reqwest_client_with_ua("scraper example 123")
		.context("Failed to create reqwest client")?;
	let base_url = Arc::from(
		Url::parse(&cli.base_url).context("Failed to parse base url")?,
	);
	let builds_endpoint = format!("{base_url}builds");
	dbg!(&builds_endpoint);

	let res = client
		.get(&builds_endpoint)
		.send()
		.await
		.context("Failed to send builds get request")?
		.bytes()
		.await
		.context("Failed to read builds bytes")?;
	let build_list: BuildList = rmp_serde::from_slice(&res)
		.context("Failed to deserialize BuildList")?;
	let mut js = JoinSet::new();
	for build_bytes in build_list.builds {
		let base_url = Arc::clone(&base_url);
		let client = Arc::clone(&client);
		let target_dir = Arc::clone(&target_dir);
		js.spawn(async move {
			let build_bytes_raw = zstd::decode_all(&*build_bytes)
				.context("Failed to decompress build bytes")?;
			let build_metadata: explorer_types::BundleMetadata =
				rmp_serde::from_slice(&build_bytes_raw)
					.context("Failed to deserialize BundleMetadata")?;
			let build_dir = target_dir.join(&build_metadata.build_hash);
			fs::create_dir(&build_dir)
				.await
				.with_context(|| {
					format!(
						"Failed to create build dir {}",
						build_dir.display()
					)
				})?;
			fs::write(build_dir.join(METADATA_FILE_NAME), build_bytes)
				.await
				.context("Failed to write build metadata")?;
			let full_bundle_data = client
				.get(format!(
					"{base_url}build/{}/full",
					build_metadata.build_hash
				))
				.send()
				.await
				.with_context(|| {
					format!(
						"Failed to get full data for {}",
						build_metadata.build_hash
					)
				})?
				.bytes()
				.await
				.with_context(|| {
					format!(
						"Failed to read full data bytes for {}",
						build_metadata.build_hash
					)
				})?;
			fs::write(build_dir.join(DATA_FILE_NAME), full_bundle_data)
				.await
				.context("Failed to write full build data")?;
			eprintln!("Done with build {}", build_metadata.build_hash);

			anyhow::Ok(())
		});
	}
	while let Some(res) = js.join_next().await {
		res.context("Failed to join task")??;
	}
	Ok(())
}
