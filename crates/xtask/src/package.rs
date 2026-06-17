use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

pub mod extension;

#[derive(Args)]
pub struct Command {
	#[command(subcommand)]
	/// The part of this project to package
	target: Target,
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match &self.target {
			Target::Extension(c) => c.run(),
		}
	}
}

#[derive(Subcommand)]
enum Target {
	/// Package the `VencordCompanion` `VSCode` extension as a .vsix
	Extension(extension::Command),
}
