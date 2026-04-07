use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

pub mod client;
pub mod server;

#[derive(Args)]
pub struct Command {
	#[command(subcommand)]
	/// The part of this project to run
	target: Target,
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match &self.target {
			Target::Server(c) => c.run(),
			Target::Client(c) => c.run(),
		}
	}
}

#[derive(Subcommand)]
enum Target {
	/// Run the explorer server
	Server(server::Command),
	/// Run the client site
	Client(client::Command),
}
