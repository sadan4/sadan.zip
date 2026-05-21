use crate::{Runnable, build};
use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument, warn};

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {
	#[instrument]
	fn generate_libsadancore_types() -> Result<()> {
		info!("Generating libsadancore types...");
		let cmd = build::client::Command {
			debug: true,
			local_server: true,
			no_minify_ssr: false,
			sub_target: None,
		};
		cmd.build_wasm()
			.context("Failed to generate libsadancore types")?;
		Ok(())
	}
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		Self::generate_libsadancore_types()?;
		Ok(())
	}
}
