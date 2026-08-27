use std::{
	fs,
	io::{self, Seek as _},
};

use anyhow::{Context, bail};
use clap::Args;
use serde_json::Value;
use tracing::info;

use crate::Runnable;

#[derive(Debug, Clone, Args)]
pub struct Command {
	/// Url to fetch the discord intl mappings from
	#[arg()]
	url: String,
}

type JsonMap<'a> = serde_json::Map<String, Value>;

const MAPPINGS_PATH: &str = "src/utils/discordI18n/key-mappings.json";

impl Runnable for Command {
	fn run(&self) -> anyhow::Result<()> {
		let new_map_raw = reqwest::blocking::get(&self.url)
			.with_context(|| format!("Failed to fetch {:?}", self.url))?
			.bytes()
			.context("Failed to read response body")?;
		let orig_map_file = fs::OpenOptions::new()
			.read(true)
			.write(true)
			.open(MAPPINGS_PATH)
			.with_context(|| format!("Failed to open {MAPPINGS_PATH:?}"))?;
		let mut orig_map_file = io::BufReader::new(orig_map_file);
		let orig_map: JsonMap = serde_json::from_reader(&mut orig_map_file)
			.context("Failed to deserialize original mappings")?;
		let new_map: JsonMap = serde_json::from_slice(&new_map_raw)
			.context("Failed to deserialize new mappings")?;
		let mut to_add = Vec::new();
		for (key, value) in new_map {
			match value {
				Value::String(s) => {
					if !orig_map.contains_key(&key) {
						to_add.push((key, s));
					}
				}
				other => {
					bail!("Unexpected value for key {key:?}: {other:?}");
				}
			}
		}
		if to_add.is_empty() {
			info!("No new mappings to add");
			return Ok(());
		}
		info!("Adding new mappings: {:#?}", to_add);
		info!("{} new mappings", to_add.len());
		let mut writer = io::BufWriter::new(orig_map_file.into_inner());
		writer
			.rewind()
			.context("fseek(0) original mappings")?;
		let mut orig_map = orig_map;
		for (key, value) in to_add {
			if orig_map.contains_key(&key) {
				unreachable!("Key {:?} should not exist in original map", key);
			}
			orig_map.shift_insert(0, key, Value::String(value));
		}
		serde_json::to_writer_pretty(&mut writer, &orig_map)
			.context("Failed to write updated mappings")?;
		// into_inner flushes
		let writer = writer
			.into_inner()
			.context("Failed to flush updated mappings")?;
		drop(writer);
		info!("Updated mappings written to {:?}", MAPPINGS_PATH);
		Ok(())
	}
}
