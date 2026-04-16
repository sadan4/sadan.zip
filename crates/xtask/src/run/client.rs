use crate::{Runnable, build::server::DepsMode};
use anyhow::Result;
use clap::{Args, ValueEnum};

#[derive(Args)]
pub struct Command {
	#[arg(long, default_value_t = false)]
	release: bool,
	#[arg(long, default_value_t = false)]
	/// Start the explorer server along with the client
	with_server: bool,
	#[command(flatten)]
	deps_mode: DepsMode,
	#[arg(value_enum, default_value_t)]
	target: Target,
}

#[derive(Default, Debug, Clone, Copy, ValueEnum)]
enum Target {
	Wasm,
	#[default]
	Js,
}

impl Command {
	#[expect(dead_code)]
	fn run_client(&self) -> Result<()> {
		todo!()
	}
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		todo!();
	}
}
