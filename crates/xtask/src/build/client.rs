use std::process;

use crate::{Runnable, util::cmd::CommandExt as _};
use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct Command {
	/// Build the site in release mode.
	///
	/// You should not pass or change this flag
	#[arg(long, default_value_t = true)]
	pub release: bool,
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
			.arg_if(!self.release, "--dev")
			.run()
			.context("Failed to build libsadancore")
	}
	pub fn build_vite(&self) -> Result<()> {
		assert!(self.release, "Vite can only build for release");
		process::Command::npx("vite")
			.context("Failed to find vite binary")?
			.arg("build")
			.run()
			.context("Failed to build site with vite")
	}
	pub fn minify_ssr(&self) -> Result<()> {
		assert!(self.release, "Minification can only be done for release");
		process::Command::new("node")
			.arg("scripts/minifySsr.ts")
			.run()
			.context("Failed to minify site SSR")
	}
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		info!("Building libsadancore...");
		self.build_wasm()?;
		info!("Building site with vite...");
		self.build_vite()?;
		if self.release {
			info!("Minifying site SSR");
			self.minify_ssr()?;
		}
		info!("Finished building site");
		Ok(())
	}
}
