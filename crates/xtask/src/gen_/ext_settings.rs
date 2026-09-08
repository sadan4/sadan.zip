use crate::{Runnable, util::cmd::CommandExt};
use anyhow::{Context, Result};
use clap::Args;
use std::{path, process};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {}

const INPUT_PATH: &str = "packages/VencordCompanion/package.json";

const OUT_PATH: &str = "packages/VencordCompanion/src/Settings.ts";

// TODO: move to rust + oxc?
impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let script_path = path::absolute("scripts/codegen/vscExtSettings.ts")
			.context("extension settings gen path")?;
		info!("Generating extension settings");
		process::Command::tsx(script_path)
			.arg("--packageJson")
			.arg(INPUT_PATH)
			.arg("--outPath")
			.arg(OUT_PATH)
			.run()
			.context("running extension settings gen script")?;
		info!("Successfully generated extension settings");
		Ok(())
	}
}
