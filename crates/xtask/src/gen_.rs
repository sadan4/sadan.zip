use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

mod indent_cache;
mod syntax;

#[derive(Args)]
pub struct Command {
	#[command(subcommand)]
	/// The thing to generate
	target: Target,
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match &self.target {
			Target::IndentCache(c) => c.run(),
			Target::Syntax(c) => c.run(),
		}
	}
}

#[derive(Subcommand)]
enum Target {
	/// Generate the indent cache for `crates/pretty_printer/src/formatted_content_builder.rs`
	IndentCache(indent_cache::Command),
	/// Generate syntax highlighting theme and language definitions for reporter
	Syntax(syntax::Command),
}
