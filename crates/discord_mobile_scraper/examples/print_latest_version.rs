use anyhow::Result;
use discord_mobile_scraper::get_latest_version;

#[tokio::main]
async fn main() -> Result<()> {
	let version = get_latest_version().await?;
	println!("{version:#?}");
	Ok(())
}
