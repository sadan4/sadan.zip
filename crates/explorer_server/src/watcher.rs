use std::{fs, time::Duration};

use anyhow::Result;
use napi::bindgen_prelude::tracing::instrument;
use reqwest::Response;
use tracing::{error, info, trace};

use crate::util::{get_build_path, is_build_downloaded};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Channel {
    Stable,
    Canary,
}

impl Channel {
    const fn get_app_url(self) -> &'static str {
        match self {
            Self::Stable => "https://discord.com/app",
            Self::Canary => "https://canary.discord.com/app",
        }
    }
}

const BUILD_ID_HEADER: &str = "x-build-id";

#[derive(Debug)]
struct Build {
    response: Response,
    build_id: String,
}

#[instrument]
async fn get_build(channel: Channel) -> Result<Option<Build>> {
    let app_url = channel.get_app_url();
    let resp = reqwest::get(app_url).await?;
    let Some(build_id) = resp.headers().get(BUILD_ID_HEADER) else {
        return Ok(None);
    };
    if build_id.is_empty() {
        return Ok(None);
    }
    let build_id = build_id.to_str()?.to_string();
    if is_build_downloaded(&build_id)? {
        trace!("build not changed");
        return Ok(None);
    }
    let build_path = get_build_path(&build_id)?;
    fs::create_dir_all(build_path)?;
    Ok(Some(Build {
        build_id,
        response: resp,
    }))
}

pub async fn start_watcher(handle: impl Fn(String)) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        match get_build(Channel::Stable).await {
            Ok(Some(build)) => {
                info!("new stable build: {}", build.build_id);
                handle(build.build_id);
            }
            Ok(None) => {}
            Err(e) => {
                error!("failed to get stable build: {e}");
            }
        }
    }
}
