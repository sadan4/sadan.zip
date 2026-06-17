use crate::{Runnable, util::cmd::CommandExt};
use anyhow::{Context, Result};
use clap::Args;
use std::{fmt::Write as _, fs, path, process};
use tracing::{info, warn};

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {}

const MAX_CACHED_INDENT_LEVEL: u8 = 20;

fn make_indent_cache(indent_size: u8) -> String {
	let mut out = String::new();
	let base_indent_str = if indent_size == 0 {
		"\\t".to_string()
	} else {
		" ".repeat(indent_size as _)
	};
	write!(out, "&[").unwrap();
	for indent_level in 0..=MAX_CACHED_INDENT_LEVEL {
		let indent_str = base_indent_str.repeat(indent_level as _);
		write!(out, r#""{indent_str}","#).unwrap();
	}
	out.pop(); // remove trailing comma
	out.push(']');
	out
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let mut out = String::new();
		write!(out, "pub const INDENT_CACHE: &[&[&str]] = &[").unwrap();
		for indent_size in 0..=8 {
			#[expect(
				clippy::write_with_newline,
				reason = "we want to be explicit here"
			)]
			write!(out, "\t{},\n", make_indent_cache(indent_size)).unwrap();
		}
		out.pop(); // remove trailing newline
		out.pop(); // remove trailing comma
		out.push_str("\n];\n");
		let cache_path =
			path::absolute("crates/pretty_printer/src/indent_cache.rs")
				.context("indent cache file path")?;
		fs::write(&cache_path, out).context("writing indent cache file")?;
		if let Err(e) = process::Command::cargo("+nightly")?
			.arg("fmt")
			.arg("--")
			.arg(&cache_path)
			.status()
		{
			warn!("Failed to format indent cache file: {e:?}");
		}
		info!("Successfully generated indent cache");
		Ok(())
	}
}
