mod err;
mod fetcher;
mod reporter;
mod util;
mod vc;
use anyhow::{Result, bail};
use clap::{CommandFactory as _, Parser};
use clap_complete::Shell;
use derive_more::{Constructor, From, Into};
use indicatif::ProgressBar;
use itertools::Itertools;
use miette::{Diagnostic, MietteHandlerOpts, NamedSource, Report, SourceCode};
use oxc::diagnostics::OxcDiagnostic;
use std::env::args;
use std::{io, path::Path, process, sync::Arc, time::Instant};
use terminal_size::terminal_size;
use tracing::{error, info, warn};

use crate::err::printer::GraphicalReportHandler;
use crate::{
    fetcher::{FetchOpts, fetch_build},
    reporter::{Msg, ReporterError, report_broken_patches},
    vc::{Plugin, VencordOpts, collect_patches},
};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(flatten)]
    vc_opts: VencordOpts,
    #[command(flatten)]
    fetch_opts: FetchOpts,
    /// If true, will dump the contents of the module, before any transformations, to `$PWD/{module_id}.js` whenever a module is involved in an error
    #[arg(long, default_value_t = false)]
    dump_on_error: bool,
    /// Generate shell completions
    #[arg(long, value_enum)]
    completions: Option<Shell>,
}

fn main() {
    dbg!(args().collect_vec());
    tracing_subscriber::fmt().init();
    miette::set_hook(Box::new(|_| {
        Box::new(
            GraphicalReportHandler::new()
                .with_width(terminal_size().map_or(80, |s| s.0.0 as usize))
                .with_cause_chain(),
        )
    }))
    .expect("Failed to set miette hook");
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
        Cli::command()
            .print_long_help()
            .expect("Failed to print help");
        bail!(
            "The passed vencord root dir {} doesn't look like a valid vencord root directory.",
            cli.vc_opts.vencord_dir.display()
        );
    }
    let patches_fut = tokio::spawn(async move { collect_patches(cli.vc_opts).await });
    let target_build_fut = tokio::spawn(async move { fetch_build(cli.fetch_opts).await });
    let (plugins, target_build) = tokio::join!(patches_fut, target_build_fut);
    let plugins = Arc::new(plugins??);
    let target_build = Arc::new(target_build??);
    let num_modules = target_build.modules.len();
    let bar = ProgressBar::new(num_modules as u64);
    info!("Starting reporter");
    let start = Instant::now();
    let plugins2 = plugins.clone();
    let mut rx = report_broken_patches(target_build.clone(), plugins2);

    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::Progress => {
                bar.inc(1);
            }
            Msg::Done(res) => {
                bar.suspend(|| match res {
                    Err(e) => {
                        error!("Reporter failed with error: {e:?}");
                    }
                    Ok(raw_time) => {
                        info!(
                            "Reporter finished in {:.2?}. (raw time: {raw_time:.2?})",
                            start.elapsed()
                        );
                    }
                });
                bar.finish();
                break;
            }
            Msg::Error(e) => {
                if cli.dump_on_error
                    && let Some(m_id) = e.module_id()
                {
                    if target_build.modules.contains_key(&m_id) {
                        let target_build = target_build.clone();
                        tokio::spawn(async move {
                            let path = format!("{m_id}.js");
                            let module = target_build.modules.get(&m_id).unwrap();
                            tokio::fs::write(path, module).await
                        });
                    } else {
                        bar.suspend(|| {
                            warn!("expected target_build to have the contents of module {m_id}");
                        });
                    }
                }
                let id = e.plugin_id();
                let path = &plugins[id as usize].entry_point;
                let source = SourceWrapper(plugins.clone(), id);
                let report = Report::new(e).with_source_code(
                    NamedSource::new(path.to_string_lossy(), source).with_language("JavaScript"),
                );
                bar.suspend(|| {
                    eprintln!("{report:?}");
                });
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

#[derive(From, Into)]
struct SourceWrapper(Arc<Vec<Plugin>>, u16);

impl SourceCode for SourceWrapper {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> std::result::Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.0[self.1 as usize].entry_source.read_span(
            span,
            context_lines_before,
            context_lines_after,
        )
    }

    fn name(&self) -> Option<&str> {
        self.0[self.1 as usize].entry_point.to_str()
    }
}
