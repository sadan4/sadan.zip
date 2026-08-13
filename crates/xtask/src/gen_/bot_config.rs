use crate::Runnable;
use anyhow::{Context, Result};
use clap::Args;
use schemars::JsonSchema;
use std::fs;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

const CONFIG_OUT_PATH: &str = "crates/bot_config/bot.config.schema.json";
const GIF_TEMPLATE_OUT_PATH: &str =
	"crates/bot_config/gif_template.schema.json";

fn mk_schema<T>() -> String
where
	T: JsonSchema,
{
	let schema = schemars::schema_for!(T);
	let mut schema_json = serde_json::to_string_pretty(&schema)
		.expect("Failed to serialize schema");
	schema_json.push('\n');
	schema_json
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		info!("Generating bot config JSON schema...");
		let config_schema = mk_schema::<bot_config::Config>();
		let gif_temlpate_schema = mk_schema::<bot_config::GifTemplateData>();
		fs::write(CONFIG_OUT_PATH, config_schema).with_context(|| {
			format!("Failed to write schema to {CONFIG_OUT_PATH}")
		})?;
		info!("Wrote {CONFIG_OUT_PATH}");
		fs::write(GIF_TEMPLATE_OUT_PATH, gif_temlpate_schema).with_context(
			|| format!("Failed to write schema to {GIF_TEMPLATE_OUT_PATH}"),
		)?;
		Ok(())
	}
}
