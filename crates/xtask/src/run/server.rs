use crate::{
	Runnable,
	build::{self},
	clean,
};
use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

#[derive(Args, Debug)]
pub struct Command {
	#[arg(short, long, default_value_t = false)]
	/// Run the server in debug mode.
	pub debug: bool,
	#[arg(short, long, default_value_t = false)]
	/// Clean the build cache of the server that stores previous scraped builds
	pub clean_cache: bool,
}

impl Runnable for Command {
	#[instrument]
	fn run(&self) -> Result<()> {
		if self.clean_cache {
			info!(
				"Cleaning build cache before starting server because flag was set"
			);
			clean::build_cache::Command
				.run()
				.with_context(|| "Failed to clean build cache")?;
		}
		info!("Running Server");
		build::server::Command {
			release: !self.debug,
		}
		.build_server("run")?;

		Ok(())
	}
}
