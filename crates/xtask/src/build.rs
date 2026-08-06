use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

pub mod bot;
pub mod client;
pub mod extension;
pub mod server;

#[derive(Args)]
pub struct Command {
	#[command(subcommand)]
	/// The part of this project to build
	target: Target,
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match &self.target {
			Target::Server(c) => c.run(),
			Target::Client(c) => c.run(),
			Target::Extension(c) => c.run(),
			Target::Bot(c) => c.run(),
		}
	}
}

#[derive(Subcommand)]
enum Target {
	/// Build the explorer server
	Server(server::Command),
	/// Build the client site
	Client(client::Command),
	/// Build the `VencordCompanion` `VSCode` extension
	Extension(extension::Command),
	/// Build the Discord bot and its qalc sandbox worker
	Bot(bot::Command),
}
