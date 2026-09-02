// mod spawn;
use anyhow::Result;
use discord_scraper::{NoProgress, make_reqwest_client, scrape_full_bundle};
use explorer_server_core::{
	Channel,
	get_build_path,
	is_build_downloaded,
	write_full_bundle,
};
use reqwest::Response;
use std::{fs, sync::Arc, time::Duration};
use tokio::time;
use tracing::{error, info, instrument, trace};

use crate::state::State;

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

async fn handle_build(c: Channel, state: &State) -> Result<()> {
	if let Some(build) = get_build(c).await? {
		info!("new {c:?} build: {}", build.build_hash);
		let state = state.clone();
		// FIXME: handle run_js_handler errs
		tokio::spawn(async move {
			let start = time::Instant::now();
			let result = async {
				let client = make_reqwest_client()?;
				let html = build.response.text().await?;
				scrape_full_bundle(
					&html,
					c,
					build.build_hash,
					client,
					Arc::new(NoProgress),
				)
				.await
			}
			.await;
			match result {
				Ok(build) => {
					let meta = build
						.metadata
						.clone()
						.unwrap_or_default();
					tokio::spawn(async move { state.add_build(meta).await });
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

pub async fn start_watcher(state: State) {
	info!("starting watcher loop");
	let mut interval = tokio::time::interval(Duration::from_mins(1));
	loop {
		interval.tick().await;
		match handle_build(Channel::Stable, &state).await {
			Ok(()) => {}
			Err(e) => {
				error!("failed to get stable build: {e}");
			}
		}
	}
}
