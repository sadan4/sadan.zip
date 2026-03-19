mod parser;
mod spawn;
use std::{fs, io, time::Duration};

use anyhow::{Context as _, Result};
use reqwest::Response;
use tracing::{error, info, instrument, trace};

use explorer_server_core::{Channel, EncodableBuild, get_build_path, is_build_downloaded};

use crate::watcher::{
    parser::{ParsedHtml, parse_html},
    spawn::{BuildParserWorker as _, DefaultBuildParserWorker},
};

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

async fn write_to_pipe(build: Build, channel: Channel, mut tx: io::PipeWriter) -> Result<()> {
    let build_hash = build.build_id;

    let ParsedHtml {
        global_env_text,
        web_js_url,
    } = parse_html(&build.response.text().await?)?;

    let eb = EncodableBuild {
        channel,
        build_hash,
        global_env_text,
        web_js_url,
    };
    rmp_serde::encode::write(&mut tx, &eb)?;
    Ok(())
}

async fn run_js_handler(build: Build, channel: Channel) -> Result<()> {
    let (rx, tx) = io::pipe()?;
    let writer_fut = tokio::spawn(write_to_pipe(build, channel, tx));
    spawn::DefaultBuildParserWorker::spawn(rx).await?;

    writer_fut
        .await
        .map_err(From::from)
        .flatten()
        .context("Failed to write build info to pipe")?;

    Ok(())
}

#[instrument]
async fn handle_build(c: Channel) -> Result<()> {
    if let Some(build) = get_build(c).await? {
        info!("new {c:?} build: {}", build.build_id);
        // FIXME: handle run_js_handler errs
        tokio::spawn(async move {
            match run_js_handler(build, c).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to spawn js handler: {e:?}");
                }
            }
        });
    }
    Ok(())
}

pub async fn start_watcher() {
    info!("setting up parser worker");
    if let Err(e) = DefaultBuildParserWorker::setup()
        .await
        .context("Failed to setup parser worker")
    {
        error!("{e:?}");
        return;
    }
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
