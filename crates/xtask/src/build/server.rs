use std::process;

use crate::{Runnable, util::cmd::CommandExt as _};
use anyhow::Result;
use clap::Args;
use tracing::{info, instrument};

#[derive(Args, Debug)]
pub struct Command {
	#[arg(short, long, default_value_t = false)]
	/// Build in release mode with optimizations.
	pub release: bool,
}
impl Command {
	#[instrument(skip(self))]
	pub fn build_server(&self, cargo_subcmd: &str) -> Result<()> {
		info!("Building server native code");
		process::Command::cargo(cargo_subcmd)?
			.arg("-p")
			.arg("explorer_server")
			.arg_if(self.release, "--release")
			.run()
	}
}

impl Runnable for Command {
	#[instrument(skip(self))]
	fn run(&self) -> Result<()> {
		info!(?self, "Building server");
		self.build_server("build")?;
		info!("Done");
		Ok(())
	}
}
