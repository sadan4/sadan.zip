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
		let script_path = path::absolute("scripts/codegen/monacoEditor.ts")
			.context("monaco editor entry gen path")?;
		info!("Generating monaco editor entry");
		process::Command::tsx(script_path)
			.run()
			.context("running monaco editor entry gen script")?;
		info!("Successfully generated client monaco editor entry");
		Ok(())
	}
}
