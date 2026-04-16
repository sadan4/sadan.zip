// mod spawn;
use anyhow::Result;
use explorer_server_core::{Channel, get_build_path, is_build_downloaded, write_full_bundle};
use reqwest::Response;
use std::{fs, time::Duration};
use tokio::time;
use tracing::{error, info, instrument, trace};

use crate::scraper::scrape_build;

const fn get_app_url(c: Channel) -> &'static str {
	match c {
		Channel::Stable => "https://discord.com/app",
		Channel::Canary => "https://canary.discord.com/app",
	}
}

const BUILD_ID_HEADER: &str = "x-build-id";

#[derive(Debug)]
pub struct Build {
	response: Response,
	build_hash: String,
}

#[instrument]
async fn get_build(channel: Channel) -> Result<Option<Build>> {
	let app_url = get_app_url(channel);
	let resp = reqwest::get(app_url).await?;
	let Some(build_id) = resp.headers().get(BUILD_ID_HEADER) else {
		return Ok(None);
	};
	if build_id.is_empty() {
		return Ok(None);
	}
	let build_hash = build_id.to_str()?.to_string();
	if is_build_downloaded(&build_hash)? {
		trace!("build not changed");
		return Ok(None);
	}
	let build_path = get_build_path(&build_hash)?;
	fs::create_dir_all(build_path)?;
	Ok(Some(Build {
		build_hash,
		response: resp,
	}))
}

// use std::io;
// use crate::scraper::html_parser::{ParsedHtml, parse_html};
// use crate::watcher::spawn::{BuildParserWorker as _, DefaultBuildParserWorker},
// use explorer_server_core::EncodableBuild;
// async fn write_to_pipe(
// 	build: Build,
// 	channel: Channel,
// 	mut tx: io::PipeWriter,
// ) -> Result<()> {
// 	let build_hash = build.build_hash;

// 	let ParsedHtml {
// 		global_env_text,
// 		web_js_url,
// 	} = parse_html(&build.response.text().await?)?;

// 	let eb = EncodableBuild {
// 		channel,
// 		build_hash,
// 		global_env_text,
// 		web_js_url,
// 	};
// 	rmp_serde::encode::write(&mut tx, &eb)?;
// 	Ok(())
// }

// async fn run_js_handler(build: Build, channel: Channel) -> Result<()> {
// 	let (rx, tx) = io::pipe()?;
// 	let writer_fut = tokio::spawn(write_to_pipe(build, channel, tx));
// 	spawn::DefaultBuildParserWorker::spawn(rx).await?;

// 	writer_fut
// 		.await
// 		.map_err(From::from)
// 		.flatten()
// 		.context("Failed to write build info to pipe")?;

// 	Ok(())
// }

#[instrument]
async fn handle_build(c: Channel) -> Result<()> {
	if let Some(build) = get_build(c).await? {
		info!("new {c:?} build: {}", build.build_hash);
		// FIXME: handle run_js_handler errs
		tokio::spawn(async move {
			let start = time::Instant::now();
			match scrape_build(build.response, c, build.build_hash).await {
				Ok(build) => {
					if let Err(e) = write_full_bundle(&build) {
						error!("Failed to write full bundle: {e:?}");
					}
					info!(
						"finished handling {c:?} build in {:?}",
						start.elapsed()
					);
				}
				Err(e) => {
					info!(
						"finished handling {c:?} build in {:?}",
						start.elapsed()
					);
					error!("Failed to spawn js handler: {e:?}");
				}
			}
		});
	}
	Ok(())
}

#[allow(clippy::cognitive_complexity, reason = "broken and counts macros")]
pub async fn start_watcher() {
	// info!("setting up parser worker");
	// if let Err(e) = DefaultBuildParserWorker::setup()
	// 	.await
	// 	.context("Failed to setup parser worker")
	// {
	// 	error!("{e:?}");
	// 	return;
	// }
	info!("starting watcher loop");
	let mut interval = tokio::time::interval(Duration::from_secs(5));
	loop {
		interval.tick().await;
		match handle_build(Channel::Stable).await {
			Ok(()) => {}
			Err(e) => {
				error!("failed to get stable build: {e}");
			}
		}
	}
}
