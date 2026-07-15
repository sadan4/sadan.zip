use crate::Runnable;
use anyhow::{Context, Result};
use clap::Args;
use smol_str::SmolStr;
use std::{collections::HashMap, fs};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

const OUT_PATH: &str = "crates/webpack_ast_parser/src/key_mappings.mpk.zst";

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let raw_json = fs::read("src/utils/discordI18n/key-mappings.json")
			.context("Failed to read key-mappings.json")?;
		let json: HashMap<SmolStr, SmolStr> =
			serde_json::from_slice(&raw_json)
				.context("Failed to parse key-mappings.json")?;
		let ser_keys = rmp_serde::to_vec(&json)
			.context("Failed to serialize key-mappings.json to MessagePack")?;
		let compressed = zstd::encode_all(&*ser_keys, 10)
			.context("Failed to compress key-mappings.json")?;
		fs::write(OUT_PATH, &compressed)
			.with_context(|| format!("Failed to write data to {OUT_PATH}"))?;
		info!("Done");
		Ok(())
	}
}
