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
		let script_path = path::absolute("scripts/codegen/monacoThemes.ts")
			.context("monaco themes gen path")?;
		info!("Generating monaco themes");
		process::Command::tsx(script_path)
			.run()
			.context("running monaco theme gen script")?;
		info!("Successfully generated client monaco themes");
		Ok(())
	}
}
