use std::process;

use crate::{Runnable, util::cmd::CommandExt as _};
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use tracing::info;

#[derive(Args)]
pub struct Command {
	/// Build the site in debug mode.
	#[arg(long, default_value_t = false)]
	pub debug: bool,
	#[arg(short, long, default_value_t = false)]
	/// Connect to the local server for the bundle explorer
	pub local_server: bool,
	#[arg(long, default_value_t = false)]
	pub no_minify_ssr: bool,
	#[arg(value_enum)]
	/// sub-target to build.
	///
	/// if not passed, builds everything (wasm and vite)
	pub sub_target: Option<SubTarget>,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum SubTarget {
	/// build the client wasm only
	Wasm,
}
impl Command {
	pub fn build_wasm(&self) -> Result<()> {
		process::Command::new("wasm-pack")
			.arg("build")
			.arg("--target")
			.arg("web")
			.arg("--scope")
			.arg("sadan4")
			.arg("crates/libsadancore")
			.arg_if(self.debug, "--dev")
			.arg_if(self.local_server, "--")
			.arg_if(self.local_server, "-F")
			.arg_if(self.local_server, "local-server")
			.run()
			.context("Failed to build libsadancore")
	}
	pub fn build_vite(&self) -> Result<()> {
		assert!(!self.debug, "Vite can only build for release");
		process::Command::npx("vite")
			.context("Failed to find vite binary")?
			.arg("build")
			.run()
			.context("Failed to build site with vite")
	}
	pub fn minify_ssr(&self) -> Result<()> {
		assert!(!self.debug, "Minification can only be done for release");
		process::Command::new("node")
			.arg("scripts/minifySsr.ts")
			.run()
			.context("Failed to minify site SSR")
	}
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match self.sub_target {
			None => {
				info!("Building libsadancore...");
				self.build_wasm()?;
				info!("Building site with vite...");
				self.build_vite()?;
				if !self.debug && !self.no_minify_ssr {
					info!("Minifying site SSR");
					self.minify_ssr()?;
				}
				info!("Finished building site");
			}
			Some(SubTarget::Wasm) => {
				info!("Building libsadancore...");
				self.build_wasm()?;
				info!("Finished building libsadancore");
			}
		}
		Ok(())
	}
}
