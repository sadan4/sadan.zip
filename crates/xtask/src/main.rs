use std::{env, io, path::Path};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use tracing::level_filters::LevelFilter;

mod build;
mod clean;
mod deps;
mod gen_;
mod run;
mod util;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct XTask {
	#[arg(long, value_enum, default_value_t = LogLevel::Info)]
	log_level: LogLevel,
	#[command(subcommand)]
	command: Command,
}

#[derive(ValueEnum, Clone, Copy)]
enum LogLevel {
	Trace,
	Debug,
	Info,
	Warn,
	Error,
	Off,
}

impl From<LogLevel> for LevelFilter {
	fn from(value: LogLevel) -> Self {
		match value {
			LogLevel::Trace => Self::TRACE,
			LogLevel::Debug => Self::DEBUG,
			LogLevel::Info => Self::INFO,
			LogLevel::Warn => Self::WARN,
			LogLevel::Error => Self::ERROR,
			LogLevel::Off => Self::OFF,
		}
	}
}

#[derive(Subcommand)]
enum Command {
	Build(build::Command),
	Clean(clean::Command),
	Run(run::Command),
	Gen(gen_::Command),
	Completions {
		#[arg(value_enum)]
		shell: Shell,
	},
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match self {
			Self::Build(cmd) => cmd.run(),
			Self::Clean(cmd) => cmd.run(),
			Self::Run(cmd) => cmd.run(),
			Self::Gen(cmd) => cmd.run(),
			// this is handled in main and we exit early
			Self::Completions { shell } => {
				clap_complete::generate(
					*shell,
					&mut XTask::command(),
					"cargo-xtask",
					&mut io::stdout(),
				);
				Ok(())
			}
		}
	}
}

trait Runnable {
	fn run(&self) -> Result<()>;
}

fn ensure_in_workspace_root() -> Result<()> {
	for file in ["package.json", "Cargo.toml", "src", "crates", ".git"] {
		if !Path::new(file).exists() {
			let cwd = env::current_dir()?;
			bail!(
				"Expected to find {file} in current directory {}. Run this command from the workspace root.",
				cwd.display()
			);
		}
	}
	Ok(())
}

fn main() -> Result<()> {
	ensure_in_workspace_root()
		.with_context(|| "Failed to locate workspace root")?;
	let xt = XTask::parse();
	tracing_subscriber::fmt()
		.with_max_level(xt.log_level)
		.init();
	xt.command.run()?;
	Ok(())
}
