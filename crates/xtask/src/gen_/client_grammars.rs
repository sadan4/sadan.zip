use crate::{Runnable, util::cmd::CommandExt};
use anyhow::{Context, Result};
use clap::Args;
use std::{path, process};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let script_path = path::absolute("scripts/codegen/grammars.ts")
			.context("grammar gen path")?;
		info!("Generating client grammars");
		process::Command::tsx(script_path)
			.run()
			.context("running grammar gen script")?;
		info!("Successfully generated client grammars");
		Ok(())
	}
}
