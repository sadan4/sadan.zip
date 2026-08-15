use std::{hint::black_box, path::PathBuf, time::Instant};

use anyhow::{Context as _, Result};
use clap::Parser;
use tracing::info;

#[derive(clap::Parser)]
struct Args {
	/// The path to the image
	path: PathBuf,
	/// Optional path to write the resulting webp to
	#[arg(short, long)]
	out: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
	bot::install_tracing();
	let args = Args::parse();
	let image = bot::util::Image::from_path(&args.path)
		.await
		.context("Failed to load image")?;
	let start = Instant::now();
	let bytes = bot::cmds::image::spin_image_no_clip(&image)
		.context("Failed to spin image")?;
	info!(
		"Spun image in {:?}, {}",
		start.elapsed(),
		bot::util::FormatBytes(bytes.len()),
	);
	if let Some(out) = args.out {
		std::fs::write(&out, &bytes).with_context(|| {
			format!("Failed to write webp to {}", out.display())
		})?;
	}
	black_box(bytes);
	Ok(())
}
