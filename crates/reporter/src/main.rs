mod fetcher;
mod util;
mod vc;
use anyhow::{Context as _, Result, bail};
use clap::{CommandFactory as _, Parser, error};
use clap_complete::Shell;
use std::{
    env, io,
    path::{Path, PathBuf},
    process,
};
use tokio::fs;
use tracing::{error, info};

use crate::{
    fetcher::{BuildFilter, fetch_build},
    vc::{VencordOpts, collect_patches},
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
    #[cfg(debug_assertions)]
    unsafe {
        env::set_var("RUST_BACKTRACE", "1");
    };
    tracing_subscriber::fmt().init();
    async_main();
}

#[tokio::main]
async fn async_main() {
    let cli = Cli::parse();
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
    let (patches, target_build) = tokio::join!(patches_fut, target_build_fut);
    let patches = patches??;
    let target_build = target_build??;
    dbg!(patches);
    dbg!(target_build);

    todo!()
}

fn is_likely_vencord_dir(path: &Path) -> bool {
    ["src/plugins/_core", "src/Vencord.ts"]
        .iter()
        .all(|p| path.join(p).exists())
}
