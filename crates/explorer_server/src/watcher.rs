use std::{fs, io, process::Stdio, time::Duration};

use anyhow::{Result, bail};
use reqwest::Response;
use tokio::process::Command;
use tracing::{error, info, instrument, trace};

use explorer_server_core::{
    Channel, EncodableBuild, Sha1Hash, get_build_path, is_build_downloaded,
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

static JS_PATH: &str = "asd";

#[instrument]
async fn run_js_handler(build: Build, channel: Channel) -> Result<()> {
    // hoist into function to specify result type for `?` operator
    async fn do_pipe_write(build: Build, channel: Channel, mut tx: io::PipeWriter) -> Result<()> {
        let build_hash = Sha1Hash::try_from(build.build_id.as_str())?;
        let html = build.response.text().await?;
        let eb = EncodableBuild {
            channel,
            build_hash,
            html,
        };
        eb.encode(&mut tx)?;
        Ok(())
    }
    let (rx, tx) = io::pipe()?;
    let pipe_write_task = tokio::spawn(async move { do_pipe_write(build, channel, tx).await });
    let status = Command::new("node")
        .arg(JS_PATH)
        .stdin(rx)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            bail!("js process failed with status {s}");
        }
        Err(e) => {
            bail!("failed to run js handler: {e}");
        }
    }
    match pipe_write_task.await {
        Ok(Ok(())) => {},
        Ok(Err(e)) => {
            bail!("writing to js handler stdin failed: {e}");
        }
        Err(e) => {
            bail!("failed to join js handler task: {e}");
        }
    }
    Ok(())
}

async fn handle_build(c: Channel) -> Result<()> {
    if let Some(build) = get_build(c).await? {
        info!("new {c:?} build: {}", build.build_id);
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
