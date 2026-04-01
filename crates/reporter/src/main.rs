mod diag;
mod err;
mod fetcher;
mod reporter;
mod util;
mod vc;
use anyhow::{Result, bail};
use clap::{CommandFactory as _, Parser};
use clap_complete::Shell;
use derive_more::{From, Into};
use indicatif::MultiProgress;
use miette::{Diagnostic, NamedSource, Report, Severity::Warning, SourceCode};
use std::{
	io,
	mem,
	path::Path,
	process,
	sync::{Arc, LazyLock},
	time::Instant,
};
use terminal_size::terminal_size;
use tracing::{Level, error, warn};
use tracing_subscriber::util::SubscriberInitExt;

use crate::{
	err::printer::GraphicalReportHandler,
	fetcher::{FetchOpts, fetch_build},
	reporter::{Msg, report_broken_patches},
	util::Stage,
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
	/// If true, will not print reporter warnings.
	///
	/// This is not the same thing as a patch being noWarn
	#[arg(long, default_value_t = false)]
	no_warnings: bool,
	/// Generate shell completions
	#[arg(long, value_enum)]
	completions: Option<Shell>,
}

#[derive(From)]
struct MultiProgressWrapper(MultiProgress);

impl io::Write for MultiProgressWrapper {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.0
			.suspend(|| io::stderr().lock().write(buf))
	}

	// no need to flush stderr, rust doesn't either
	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

static GLOBAL_BAR: LazyLock<MultiProgress> = LazyLock::new(MultiProgress::new);

fn install_tracing() {
	use tracing_subscriber::{
		EnvFilter,
		fmt,
		layer::SubscriberExt as _,
		registry,
	};
	// dbg!(args().collect_vec());
	let filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| {
			EnvFilter::builder()
				.with_default_directive(Level::DEBUG.into())
				.parse("")
		})
		.unwrap();
	let fmt_layer = fmt::layer()
		.with_writer(|| MultiProgressWrapper::from(GLOBAL_BAR.clone()));
	registry()
		.with(filter_layer)
		.with(fmt_layer)
		.init();
}

fn main() {
	install_tracing();
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
	let cli = Cli::parse();
	// if cli.vc_opts.vencord_dir == env::current_dir().unwrap() {
	//     cli.vc_opts.vencord_dir = env::home_dir().unwrap().join("dev").join("Vencord");
	// }
	if let Some(shell) = cli.completions {
		clap_complete::generate(
			shell,
			&mut Cli::command(),
			"reporter",
			&mut io::stdout(),
		);
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
	let bars = GLOBAL_BAR.clone();
	let patches_bar =
		Stage::new("Collecting Patches: ", None).and_attach(&bars);
	let fetch_bar =
		Stage::new("Resolving target build", None).and_attach(&bars);
	// we need to keep the progress bars alive so that .suspend works properly
	// SEE: https://github.com/console-rs/indicatif/issues/594
	mem::forget(patches_bar.clone());
	mem::forget(fetch_bar.clone());
	// FIXME: don't wrap in spawn
	let patches_fut =
		tokio::spawn(
			async move { collect_patches(cli.vc_opts, patches_bar).await },
		);
	let target_build_fut =
		tokio::spawn(
			async move { fetch_build(cli.fetch_opts, fetch_bar).await },
		);
	let (plugins, target_build) = tokio::join!(patches_fut, target_build_fut);
	let plugins = Arc::new(plugins??);
	let target_build = Arc::new(target_build??);
	let start = Instant::now();
	let plugins2 = plugins.clone();
	let mut rx = report_broken_patches(target_build.clone(), plugins2);

	while let Some(msg) = rx.recv().await {
		match msg {
			Msg::RequestProgressBar(tx) => {
				tx.send(bars.clone()).unwrap();
			}
			Msg::Done(res) => {
				match res {
					Err(e) => {
						bars.println(format!(
							"Reporter failed with error: {e:?}"
						))
						.unwrap();
					}
					Ok(raw_time) => {
						bars.println(format!(
							"Reporter finished in {:.2?}. (raw time: {raw_time:.2?})",
							start.elapsed()
						))
						.unwrap();
					}
				}
				break;
			}
			Msg::Error(e) => 'm: {
				let e = if (e.severity() == Some(Warning) && cli.no_warnings)
					|| e.is_no_warn()
				{
					break 'm;
				} else {
					e
				};
				if cli.dump_on_error
					&& let Some(m_id) = e.module_id()
				{
					if target_build.modules.contains_key(&m_id) {
						let target_build = target_build.clone();
						tokio::spawn(async move {
							let path = format!("{m_id}.js");
							let module =
								target_build.modules.get(&m_id).unwrap();
							tokio::fs::write(path, module).await
						});
					} else {
						warn!(
							"expected target_build to have the contents of module {m_id}"
						);
					}
				}
				let id = e.plugin_id();
				let path = &plugins[id as usize].entry_point;
				let source = SourceWrapper(plugins.clone(), id);
				let report = Report::new(e).with_source_code(
					NamedSource::new(path.to_string_lossy(), source)
						.with_language("JavaScript"),
				);
				bars.suspend(|| {
					eprintln!("{report:?}");
				});
			}
		}
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
	) -> std::result::Result<
		Box<dyn miette::SpanContents<'a> + 'a>,
		miette::MietteError,
	> {
		self.0[self.1 as usize]
			.entry_source
			.read_span(span, context_lines_before, context_lines_after)
	}

	fn name(&self) -> Option<&str> {
		self.0[self.1 as usize]
			.entry_point
			.to_str()
	}
}
