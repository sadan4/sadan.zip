use std::{
	io::{self, Write},
	process,
};

use anyhow::{Context as _, Result, bail};
use clap::{CommandFactory as _, Parser};
use reporter::{
	err::printer::GraphicalReportHandler,
	util::{MultiProgressWrapper, Stage},
	vc::{self, VencordOpts},
};
use terminal_size::{Width, terminal_size};
use tracing::{error, info};
use tracing_subscriber::util::SubscriberInitExt as _;

#[derive(Parser, Clone)]
#[command(version, about)]
struct Cli {
	#[command(flatten)]
	vc_opts: VencordOpts,
	#[arg(short = 's', long, default_value_t = false)]
	/// include source code in serialization
	with_source_code: bool,
}

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
		.with_writer(io::stderr);
	registry()
		.with(filter_layer)
		.with(fmt_layer)
		.init();
}

#[tokio::main]
async fn async_main() {
	match run().await {
		Err(e) => {
			error!("{e:?}");
			process::exit(1);
		}
		Ok(code) => process::exit(i32::from(code)),
	}
}

async fn run() -> Result<i8> {
	let cli = Cli::parse();
	if !vc::is_likely_vencord_dir(&cli.vc_opts.vencord_dir) {
		Cli::command()
			.print_long_help()
			.expect("Failed to print help");
		bail!(
			"The passed vencord root dir {} doesn't look like a valid vencord root directory.",
			cli.vc_opts.vencord_dir.display()
		);
	}
	let bars = MultiProgressWrapper::default();
	let patches_bar =
		Stage::new("Collecting Patches: ", None).and_attach(&bars);
	let mut plugins = vc::collect_patches(cli.vc_opts, patches_bar)
		.await
		.context("Failed to collect patches")?;
	if !cli.with_source_code {
		for p in &mut plugins {
			p.entry_source.clear();
		}
	}
	let raw_plugins = rmp_serde::to_vec_named(&plugins)
		.context("Failed to serialize plugins")?;
	let zst_compressed_plugins = zstd::encode_all(&raw_plugins[..], 13)
		.context("Failed to compress plugin data")?;
	io::stdout()
		.write_all(&zst_compressed_plugins)
		.context("Failed to write data to stdout")?;
	info!("Serialized {} plugins", plugins.len());
	Ok(0)
}
fn main() {
	install_tracing();
	miette::set_hook(Box::new(|_| {
		Box::new(
			GraphicalReportHandler::new()
				.with_width(
					terminal_size()
						.map_or(80, |(Width(width), _)| width as usize),
				)
				.with_cause_chain(),
		)
	}))
	.expect("Failed to set miette hook");
	async_main();
}
