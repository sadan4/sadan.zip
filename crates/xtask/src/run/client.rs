use std::process;

use crate::{
	Runnable,
	build::{self},
	util::cmd::CommandExt,
};
use anyhow::Result;
use clap::{Args, ValueEnum};
use tracing::info;

#[derive(Args)]
pub struct Command {
	#[arg(long, default_value_t = false)]
	release: bool,
	#[arg(long, default_value_t = false)]
	/// Start the explorer server along with the client
	with_server: bool,
}

#[derive(Default, Debug, Clone, Copy, ValueEnum)]
enum Target {
	Wasm,
	#[default]
	Js,
}

impl Command {
	fn run_client(&self) -> Result<()> {
		let guh = build::client::Command {
			release: self.release,
		};
		info!("Building client wasm");
		guh.build_wasm()?;
		if self.release {
			info!("Building client for preview");
			guh.build_vite()?;
			info!("Starting preview server");
			process::Command::npx("vite")?
				.arg("preview")
				.run()?;
		} else {
			info!("Starting vite dev server");
			process::Command::npx("vite")?
				.arg("dev")
				.run()?;
			info!("Vite dev server stopped");
		}
		Ok(())
	}
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		if self.with_server {
			todo!("run server");
		}
		self.run_client()
	}
}
