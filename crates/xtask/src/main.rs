use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::{env, path::Path};
use tracing::level_filters::LevelFilter;

mod build;
mod clean;
mod deps;
mod gen_;
mod package;
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
	Package(package::Command),
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match self {
			Self::Build(cmd) => cmd.run(),
			Self::Clean(cmd) => cmd.run(),
			Self::Run(cmd) => cmd.run(),
			Self::Gen(cmd) => cmd.run(),
			Self::Package(cmd) => cmd.run(),
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
