use std::{fs::File, path::Path, sync::Arc};

use anyhow::{Context, Result};
use explorer_server_core::Channel;
use explorer_types::FullBundle;
use insta::assert_ron_snapshot;
use reporter::{
	reporter::{Msg, report_broken_patches},
	util::MultiProgressWrapper,
	vc,
};
use serde::de::DeserializeOwned;
use tokio::task::{JoinHandle, spawn_blocking};

#[tokio::test]
async fn reporter() {
	const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
	let test_data_root = Path::new(CRATE_ROOT)
		.join("tests")
		.join("data");
	let patches_path = test_data_root.join("patches.mpk.zst");
	let bundle_path =
		test_data_root.join("5f9036bea3bd644a3e7f9fed68a5e30573bd4732.mpk.zst");
	let patches_fut: JoinHandle<Result<_>> = spawn_blocking(move || {
		let mut plugins = read_mpk_zst_file::<Vec<vc::Plugin>>(&patches_path)?;
		vc::bind_plugin_ids(&mut plugins);
		vc::compile_plugin_regexes(&mut plugins);
		Result::Ok(plugins)
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
	let modules = Arc::new(bundle.modules);
	let plugins = Arc::new(plugins);
	let mut rx = report_broken_patches(
		Channel::Stable,
		modules.clone(),
		plugins.clone(),
	);
	let mut errs = Vec::new();
	let bar = MultiProgressWrapper::test_bar();
	while let Some(msg) = rx.recv().await {
		match msg {
			Msg::RequestProgressBar(sender) => {
				sender.send(bar.clone()).unwrap();
			}
			Msg::Error(mut err) => {
				err.sort_inner_data();
				errs.push(err);
			}
			Msg::Done(duration) => {
				duration.expect("reporter failed");
				break;
			}
		}
	}
	errs.sort_unstable();
	assert_ron_snapshot!(errs);
}

fn read_mpk_zst_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
	let bundle_file = File::open(path).context("Failed to open file")?;
	let zst_data = zstd::decode_all(bundle_file);
	let bundle: T = rmp_serde::from_slice(&zst_data?)
		.context("Failed to deserialize file")?;
	Ok(bundle)
}
