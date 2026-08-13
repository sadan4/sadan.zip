use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

pub mod extension;
pub mod qalc_py;

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
			Target::QalcPy(c) => c.run(),
		}
	}
}

#[derive(Subcommand)]
enum Target {
	/// Package the `VencordCompanion` `VSCode` extension as a .vsix
	Extension(extension::Command),
	/// Package the `qalc_sbox_py` module + stubs into a single wheel
	QalcPy(qalc_py::Command),
}
