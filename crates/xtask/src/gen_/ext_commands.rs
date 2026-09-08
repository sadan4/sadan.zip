use crate::{Runnable, util::cmd::CommandExt};
use anyhow::{Context, Result};
use clap::Args;
use serde::Deserialize;
use serde_json::Value;
use std::{fs, io::{self, BufRead as _}, path, process};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {}

const PACKAGE_PATH: &str = "packages/VencordCompanion/package.json";

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let (out, out_writer) =
			io::pipe().context("creating pipe for gen_cmds output")?;
		let cmd = process::Command::cargo("run")?
			.arg("--example")
			.arg("get_cmds")
			.stdout(out_writer)
			.spawn()
			.context("spawning get_cmds example")?;
		let out = io::BufReader::new(out);
		let mut cmds = Vec::new();
		#[derive(Deserialize)]
		#[serde(deny_unknown_fields)]
		struct Cmd {
			name: String,
			title: String,
		}
		for line in out.lines() {
			let cmd: Cmd =
				serde_json::from_str(&line?).context("deserializing cmd")?;
			cmds.push(cmd);
		}
		let package_json = fs::File::options()
			.read(true)
			.write(true)
			.open(PACKAGE_PATH)?;
		let mut package_json: Value = serde_json::from_reader(package_json)
			.context("reading package.json")?;
		let cur_cmds = package_json.pointer_mut("contributes/commands")
			.context("getting contributes.commands from package.json")?
			.as_array_mut()
			.context("contributes.commands is not an array")?;

		Ok(())
	}
}
