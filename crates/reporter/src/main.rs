#![feature(result_option_map_or_default)]
use clap::{CommandFactory as _, Parser as _};
use derive_more::From;
use indicatif::MultiProgress;
use miette::{Result, bail};
use reporter::{
	Cli,
	cmds,
	install_miette_hook,
	util::MultiProgressWrapper,
	vc,
};
use std::{io, process, sync::LazyLock};
use tracing::error;
use tracing_subscriber::util::SubscriberInitExt;

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
			let (is_debug, is_trace) = Cli::try_parse().map_or_default(|c| {
				let lvl = c.verbose;
				(lvl >= 1, lvl >= 2)
			});
			EnvFilter::builder().parse(if cfg!(debug_assertions) || is_debug {
				[
					if is_trace { "trace" } else { "debug" },
					"h2::codec::framed_read=info",
					"h2::codec::framed_write=info",
					"h2=debug",
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
	install_miette_hook();
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
	if !vc::is_likely_vencord_dir(&cli.vc_opts.vencord_dir) {
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
	cmds::run(cli, &GLOBAL_BAR).await
}
