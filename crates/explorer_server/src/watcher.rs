use std::time::Duration;

use anyhow::Result;
use reqwest::Response;

enum Channel {
    Stable,
    Canary,
}

impl Channel {
    const fn get_app_url(&self) -> &'static str {
        match self {
            Self::Stable => "https://discord.com/app",
            Self::Canary => "https://canary.discord.com/app",
        }
    }
}

const BUILD_ID_HEADER: &str = "x-build-id";

struct Build {
    response: Response,
    build_id: String,
}

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
    Ok(Some(Build {
        build_id,
        response: resp,
    }))
}

pub async fn start_watcher(handle: impl Fn(String)) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        handle("some build hash".to_string());
    }
}
