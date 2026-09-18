use crate::{Runnable, util::cmd::CommandExt};
use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Serializer, Value, ser::PrettyFormatter};
use std::{
	fs,
	io::{self, BufRead as _, Seek as _, Write},
	process,
};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {}

const PACKAGE_PATH: &str = "packages/VencordCompanion/package.json";

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		let (out, out_writer) =
			io::pipe().context("creating pipe for gen_cmds output")?;
		process::Command::cargo("run")?
			.arg("--example")
			.arg("get_cmds")
			.stdout(out_writer)
			.spawn()
			.context("spawning get_cmds example")?;
		let out = io::BufReader::new(out);
		let mut cmds = Vec::new();
		#[derive(Deserialize, Serialize, PartialEq, PartialOrd, Eq, Ord)]
		#[serde(deny_unknown_fields)]
		struct Cmd {
			command: String,
			title: String,
		}
		for line in out.lines() {
			let mut cmd: Cmd =
				serde_json::from_str(&line?).context("deserializing cmd")?;
			cmd.title.insert_str(0, "Vencord Companion: ");
			cmds.push(cmd);
		}
		cmds.sort_unstable();
		let cmds = cmds
			.into_iter()
			.map(serde_json::to_value)
			.collect::<Result<Vec<_>, _>>()?;
		let mut package_json = fs::File::options()
			.read(true)
			.write(true)
			.open(PACKAGE_PATH)?;
		let mut json: Value = serde_json::from_reader(&mut package_json)
			.context("reading package.json")?;
		let cmds_len = cmds.len();
		*json
			.pointer_mut("/contributes/commands")
			.context("getting contributes.commands from package.json")?
			.as_array_mut()
			.context("contributes.commands is not an array")? = cmds;
		package_json
			.rewind()
			.context("fseek(0) on package.json")?;
		package_json
			.set_len(0)
			.context("ftruncate(0) on package.json")?;
		let mut ser = Serializer::with_formatter(
			&mut package_json,
			PrettyFormatter::with_indent(b"    "),
		);
		json.serialize(&mut ser)
			.context("serializing package.json")?;
		package_json
			.flush()
			.context("flushing package.json")?;
		drop(package_json);
		info!("Updated {cmds_len} commands in {PACKAGE_PATH}");
		Ok(())
	}
}
