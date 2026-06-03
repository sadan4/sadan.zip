use crate::{
	Runnable,
	gen_::{Target, client_grammars, monaco_editor, monaco_themes, ts_api},
};
use anyhow::{Context, Result, anyhow};
use clap::Args;
use std::{thread, time::Instant};
use tracing::{info, instrument};

#[derive(Args, Clone, Debug)]
pub struct Command;

impl Command {}

static CLIENT_GEN_CMDS: &[&Target] = &[
	&Target::ClientGrammars(client_grammars::Command),
	&Target::ClientMonacoThemes(monaco_themes::Command),
	&Target::ClientMonacoEntry(monaco_editor::Command),
	&Target::ClientTsApi(ts_api::Command),
];

impl Runnable for Command {
	#[instrument(skip(self))]
	fn run(&self) -> Result<()> {
		let start = Instant::now();
		// bugged. SEE: https://github.com/rust-lang/rust-clippy/issues/16012
		#[allow(clippy::redundant_iter_cloned)]
		let threads = CLIENT_GEN_CMDS
			.iter()
			.copied()
			.cloned()
			.map(|target| {
				thread::spawn(move || {
					let runner = super::Command { target };
					runner.run()
				})
			})
			.collect::<Vec<_>>();
		for t in threads {
			t.join()
				.map_err(|_| anyhow!("gen thread panicked"))?
				.context("gen thread errored")?;
		}
		info!(
			"finished generating all client code in {:?}",
			start.elapsed()
		);
		Ok(())
	}
}
