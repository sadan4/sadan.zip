use crate::Runnable;
use anyhow::{Context, Result};
use clap::Args;
use std::{fs, io};
use syntect::{
	highlighting::ThemeSet,
	parsing::{SyntaxDefinition, SyntaxSetBuilder},
};
use tracing::info;

#[derive(Args)]
pub struct Command;

impl Command {}

const INCLUDED_SYNTAXES: &[&str] =
	&[include_str!("./data/JavaScript.sublime-syntax")];

fn encode<T: serde::Serialize>(
	value: &T,
	w: &mut impl io::Write,
) -> Result<()> {
	let raw = bitcode::serialize(value).context("bitcode")?;
	let compressed = zstd::encode_all(&*raw, 15).context("zstd")?;
	w.write_all(&compressed)
		.context("write")?;
	Ok(())
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let theme = &ThemeSet::load_defaults().themes["base16-ocean.dark"];
		let mut sb = SyntaxSetBuilder::new();
		for raw_syntax in INCLUDED_SYNTAXES {
			let syntax =
				SyntaxDefinition::load_from_str(raw_syntax, true, None)
					.context("Failed to parse syntax definition")?;
			sb.add(syntax);
		}
		let ss = sb.build();

		let mut ss_file = fs::File::create(
			"crates/reporter/src/err/printer/hl/syntaxes.bin",
		)?;
		encode(&ss, &mut ss_file)?;
		let mut theme_file =
			fs::File::create("crates/reporter/src/err/printer/hl/theme.bin")?;
		encode(&theme, &mut theme_file)?;

		info!("Done");
		Ok(())
	}
}
