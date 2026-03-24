mod fetcher;
mod util;
mod vc;
mod reporter;
use anyhow::{Context as _, Result, bail};
use clap::{CommandFactory as _, Parser, error};
use clap_complete::Shell;
use indicatif::ProgressBar;
use std::{
    env, io, path::{Path, PathBuf}, process, sync::Arc, time::Instant
};
use tokio::fs;
use tracing::{error, info, warn};

use crate::{
    fetcher::{BuildFilter, fetch_build}, reporter::{Msg, report_broken_patches}, vc::{VencordOpts, collect_patches}
};

// const DEFAULT_BACKEND_URL: &str = "https://s-d-br.sadan.zip";
const DEFAULT_BACKEND_URL: &str = "http://localhost:8484";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(flatten)]
    vc_opts: VencordOpts,
    // /// Try to run reporter against the build with this number. Fails if the build can't be found.
    // build_number: Option<u32>,
    /// The backend URL to fetch builds from.
    /// You should not need to pass this in most cases.
    #[arg(long, default_value = DEFAULT_BACKEND_URL)]
    backend_url: String,
    /// Generate shell completions
    #[arg(long, value_enum)]
    completions: Option<Shell>,
}

fn main() {
    tracing_subscriber::fmt().init();
    async_main();
}

#[tokio::main]
async fn async_main() {
    let mut cli = Cli::parse();
    // if cli.vc_opts.vencord_dir == env::current_dir().unwrap() {
    //     cli.vc_opts.vencord_dir = env::home_dir().unwrap().join("dev").join("Vencord");
    // }
    if let Some(shell) = cli.completions {
        clap_complete::generate(shell, &mut Cli::command(), "reporter", &mut io::stdout());
        process::exit(0);
    }
    if let Err(e) = run(cli).await {
        error!("{e:?}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    if !is_likely_vencord_dir(&cli.vc_opts.vencord_dir) {
        bail!(
            "The passed vencord root dir {} doesn't look like a valid vencord root directory.",
            cli.vc_opts.vencord_dir.display()
        );
    }
    let patches_fut = tokio::spawn(async move { collect_patches(cli.vc_opts).await });
    let target_build_fut =
        tokio::spawn(async move { fetch_build(&cli.backend_url, BuildFilter::Latest).await });
    let (plugins, target_build) = tokio::join!(patches_fut, target_build_fut);
    let plugins = plugins??;
    let target_build = Arc::new(target_build??);
    let num_modules = target_build.modules.len();
    let bar = ProgressBar::new(num_modules as u64);
    info!("Starting reporter");
    let start = Instant::now();
    let mut rx = report_broken_patches(target_build.clone(), plugins);

    while let Some(msg) = rx.recv().await { 
        match msg {
            Msg::Progress => {
                bar.inc(1);
            }
            Msg::Done(res) => {
                if let Err(e) = res {
                    error!("Reporter failed with error: {e:?}");
                }
                bar.suspend(|| {
                    info!("Reporter finished in {:.2?}", start.elapsed());
                });
                bar.finish();
                break;
            }
            msg => {
                // dbg!(msg);
            }
        }
    }

    if cfg!(debug_assertions) && !bar.is_finished() {
        warn!("Progress bar not finished");
        bar.finish();
    }

    Ok(())
}

fn is_likely_vencord_dir(path: &Path) -> bool {
    ["src/plugins/_core", "src/Vencord.ts"]
        .iter()
        .all(|p| path.join(p).exists())
}
