use crate::Runnable;
use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

const OUT_PATH: &str = "crates/bot_config/bot.config.schema.json";

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		info!("Generating bot config JSON schema...");
		let schema = schemars::schema_for!(bot_config::Config);
		let json = serde_json::to_string_pretty(&schema)
			.context("Failed to serialize bot config schema")?;
		fs::write(OUT_PATH, format!("{json}\n"))
			.with_context(|| format!("Failed to write schema to {OUT_PATH}"))?;
		info!("Wrote {OUT_PATH}");
		Ok(())
	}
}
