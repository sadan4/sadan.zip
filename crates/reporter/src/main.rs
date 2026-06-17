mod cmds;
mod diag;
mod err;
mod fetcher;
mod reporter;
mod util;
mod vc;
use anyhow::{Result, bail};
use clap::{CommandFactory as _, Parser, ValueEnum};
use clap_complete::Shell;
use derive_more::{From, Into};
use explorer_server_core::Channel;
use indicatif::MultiProgress;
use miette::SourceCode;
use std::{
	io,
	path::Path,
	process,
	sync::{Arc, LazyLock},
};
use terminal_size::terminal_size;
use tracing::error;
use tracing_subscriber::util::SubscriberInitExt;

use crate::{
	err::printer::GraphicalReportHandler,
	fetcher::FetchOpts,
	util::MultiProgressWrapper,
	vc::{Plugin, VencordOpts},
};

#[derive(Parser, Clone)]
#[command(version, about)]
struct Cli {
	#[command(flatten)]
	vc_opts: VencordOpts,
	#[command(flatten)]
	fetch_opts: FetchOpts,
	/// Dump the contents of any module involved in an error, before any transformations, to `$PWD/{Stable, Canary}/<module_id>.js`
	#[arg(long, default_value_t = false)]
	dump_on_error: bool,
	/// Do not print reporter warnings, only print errors.
	///
	/// This is not the same thing as a patch being noWarn
	#[arg(long, default_value_t = false)]
	no_warnings: bool,
	/// Generate shell completions
	#[arg(long, value_enum)]
	completions: Option<Shell>,
	#[command(subcommand)]
	cmd: cmds::Cmd,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Branch {
	Stable,
	Canary,
}

impl From<Branch> for Channel {
	fn from(value: Branch) -> Self {
		match value {
			Branch::Stable => Self::Stable,
			Branch::Canary => Self::Canary,
		}
	}
}

#[derive(From)]
struct MultiProgressWriteWrapper(&'static MultiProgress);

impl io::Write for MultiProgressWriteWrapper {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.0
			.suspend(|| io::stderr().lock().write(buf))
	}

	// no need to flush stderr, rust doesn't either
	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

static GLOBAL_BAR: LazyLock<MultiProgressWrapper> =
	LazyLock::new(MultiProgressWrapper::default);

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
			EnvFilter::builder().parse(if cfg!(debug_assertions) {
				[
					"debug",
					"h2::codec::framed_read=info",
					"h2::codec::framed_write=info",
					"hyper_util::client::legacy::pool=info",
				]
				.join(",")
			} else {
				String::from("info")
			})
		})
		.unwrap();
	let fmt_layer = fmt::layer()
		.with_line_number(true)
		.with_ansi_sanitization(false)
		.with_ansi(true)
		.with_writer(|| MultiProgressWriteWrapper::from(GLOBAL_BAR.inner_()));
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
	match run(cli).await {
		Err(e) => {
			error!("{e:?}");
			process::exit(1);
		}
		Ok(code) => process::exit(i32::from(code)),
	}
}

async fn run(mut cli: Cli) -> Result<i8> {
	if !is_likely_vencord_dir(&cli.vc_opts.vencord_dir) {
		Cli::command()
			.print_long_help()
			.expect("Failed to print help");
		bail!(
			"The passed vencord root dir {} doesn't look like a valid vencord root directory.",
			cli.vc_opts.vencord_dir.display()
		);
	}
	cli.fetch_opts.branches.dedup();
	cli.fetch_opts.branches.sort();
	cmds::run(cli).await
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
	) -> std::result::Result<miette::MietteSpanContents<'a>, miette::MietteError>
	{
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
