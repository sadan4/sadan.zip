mod migrations;
mod server;
mod state;
mod watcher;

use clap::Parser;
use std::process;
use tokio::task::JoinSet;

use tracing::{debug, error, info, warn};

use migrations::migrate_if_needed;
use tracing_subscriber::{
	EnvFilter,
	layer::SubscriberExt,
	util::SubscriberInitExt,
};

pub use crate::state::State;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	#[arg(long, default_value_t = 8484)]
	port: u16,
	#[arg(long, default_value_t = String::from("0.0.0.0"))]
	host: String,
}

#[expect(dead_code)]
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
	let state = State::default();
	let mut tasks = JoinSet::new();
	let state_ = state.clone();
	tasks.spawn(async move {
		if let Err(e) = state_.populate_from_disk().await {
			error!("Failed to populate state from disk: {e:?}");
		}
	});
	debug!("spawned state population");
	let state_ = state.clone();
	tasks.spawn(async move {
		watcher::start_watcher(state_).await;
	});
	debug!("spawned watcher");
	let state_ = state.clone();
	tasks.spawn(async move {
		if let Err(e) =
			server::serve(&format!("{}:{}", cli.host, cli.port), state_).await
		{
			error!("Error in HTTP server: {e}");
		}
		warn!("HTTP server exited");
	});
	debug!("spawned HTTP server");
	info!("Explorer server started");
	while let Some(task) = tasks.join_next().await {
		if let Err(e) = task {
			error!("Task failed: {e}");
			process::exit(1);
		}
	}
	warn!("All tasks exited, shutting down");
}
