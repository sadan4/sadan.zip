mod parser;
use std::{fs, process::Stdio, time::Duration};

use anyhow::{Result, bail};
use reqwest::Response;
use tokio::process::Command;
use tracing::{error, info, instrument, trace};

use explorer_server_core::{Channel, EncodableBuild, get_build_path, is_build_downloaded};

use crate::watcher::parser::{ParsedHtml, parse_html};

const fn get_app_url(c: Channel) -> &'static str {
    match c {
        Channel::Stable => "https://discord.com/app",
        Channel::Canary => "https://canary.discord.com/app",
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
    let app_url = get_app_url(channel);
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

static JS_PATH: &str = "dist.server/index.js";

#[instrument]
async fn run_js_handler(build: Build, channel: Channel) -> Result<()> {
    info!("here 1");
    let build_hash = build.build_id;
    info!("here 2");
    let ParsedHtml {
        global_env_text,
        web_js_url,
    } = parse_html(&build.response.text().await?)?;
    info!("here 3");
    let eb = EncodableBuild {
        channel,
        build_hash,
        global_env_text,
        web_js_url,
    };
    info!("here 4");
    let status = Command::new("node")
        .arg(JS_PATH)
        .arg(serde_json::to_string(&eb)?)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
    info!("here 5");
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            bail!("js process failed with status {s}");
        }
        Err(e) => {
            bail!("failed to run js handler: {e}");
        }
    }
    Ok(())
}

async fn handle_build(c: Channel) -> Result<()> {
    if let Some(build) = get_build(c).await? {
        info!("new {c:?} build: {}", build.build_id);
        // FIXME: handle run_js_handler errs
        tokio::spawn(run_js_handler(build, c));
    }
    Ok(())
}

pub async fn start_watcher() {
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
