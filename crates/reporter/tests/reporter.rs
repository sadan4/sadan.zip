use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use explorer_types::FullBundle;
use reporter::vc;
use serde::de::DeserializeOwned;
use tokio::task::spawn_blocking;

#[tokio::test]
async fn reporter() {
	const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
	let test_data_root = Path::new(CRATE_ROOT)
		.join("tests")
		.join("data");
	let patches_path = test_data_root.join("patches.mpk.zst");
	let bundle_path = test_data_root.join("5f9036bea3bd644a3e7f9fed68a5e30573bd4732.mpk.zst");
	let patches_fut = spawn_blocking(move || {
		read_mpk_zst_file::<Vec<vc::Plugin>>(&patches_path)
	});
	let bundle_fut =
		spawn_blocking(move || read_mpk_zst_file::<FullBundle>(&bundle_path));
	let (plugins, bundle) = tokio::join!(patches_fut, bundle_fut);
	let plugins = plugins
		.unwrap()
		.context("Failed to read plugins")
		.unwrap();
	let bundle = bundle
		.unwrap()
		.context("Failed to read bundle")
		.unwrap();
	_ = plugins;
	_ = bundle;
}

fn read_mpk_zst_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
	let bundle_file = File::open(path).context("Failed to open file")?;
	let zst_data = zstd::decode_all(bundle_file);
	let bundle: T = rmp_serde::from_slice(&zst_data?)
		.context("Failed to deserialize file")?;
	Ok(bundle)
}
