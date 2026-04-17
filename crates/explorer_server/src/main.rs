mod migrations;
mod scraper;
mod server;
mod watcher;

use clap::Parser;
use std::process;

use tracing::{debug, error, info, warn};

use migrations::migrate_if_needed;
use tracing_subscriber::{
	EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	#[arg(long, default_value_t = 8484)]
	port: u16,
	#[arg(long, default_value_t = String::from("0.0.0.0"))]
	host: String,
}

#[allow(dead_code)]
const BIN_EXT: &str = if cfg!(windows) { ".exe" } else { "" };

fn install_tracing() {
	let filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| {
			EnvFilter::builder().parse(if cfg!(debug_assertions) {
				"trace,h2=info,hyper=info,rustls=info,reqwest::retry=debug"
			} else {
				"info"
			})
		})
		.unwrap();
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(filter_layer)
		.init();
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	install_tracing();
	info!("Starting explorer server...");
	// TODO: make async
	match migrate_if_needed() {
		Ok(()) => info!("Migrations complete"),
		Err(e) => {
			error!("Error during migration: {e}");
			error!("Exiting.");
			process::exit(1);
		}
	}
	let mut tasks = Vec::new();
	tasks.push(tokio::spawn(async move {
		watcher::start_watcher().await;
	}));
	debug!("spawned watcher");
	tasks.push(tokio::spawn(async move {
		if let Err(e) =
			server::serve(&format!("{}:{}", cli.host, cli.port)).await
		{
			error!("Error in HTTP server: {e}");
		}
		warn!("HTTP server exited");
	}));
	debug!("spawned HTTP server");
	info!("Explorer server started");
	for task in tasks {
		if let Err(e) = task.await {
			error!("Task failed: {e}");
			process::exit(1);
		}
	}
}
