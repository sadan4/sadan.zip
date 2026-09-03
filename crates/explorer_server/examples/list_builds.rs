//! Fetch the `/builds` endpoint and print the debug repr of every build's
//! metadata.

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use explorer_types::{
	BuildList,
	BundleMetadata,
	build_archive_client::BuildArchiveClient,
	google,
};
use prost::Message;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	/// base url of the explorer server
	#[arg(long, default_value_t = String::from("http://localhost:8484"))]
	base_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	let mut client = BuildArchiveClient::connect(cli.base_url)
		.await
		.context("Failed to connect to server")?;

	let mut builds = client
		.list_builds(google::protobuf::Empty {})
		.await
		.context("Failed to list builds")?
		.into_inner();

	while let Some(build) = builds
		.message()
		.await
		.context("Failed to read build from stream")?
	{
		println!("{build:#?}");
	}

	Ok(())
}
