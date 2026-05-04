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
		let script_path = path::absolute("scripts/codegen/tsPublicApi.ts")
			.context("ts public API gen path")?;
		info!("Generating ts public API");
		process::Command::tsx(script_path)
			.run()
			.context("running ts public API gen script")?;
		info!("Successfully generated ts public API");
		Ok(())
	}
}
