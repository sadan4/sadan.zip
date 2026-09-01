use std::{env, process, thread};

use crate::{
	Runnable,
	build::{self},
	util::cmd::CommandExt,
};
use anyhow::Result;
use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct Command {
	#[arg(long, default_value_t = false)]
	release: bool,
	#[arg(short, long, default_value_t = false)]
	/// Connect to the local server for the bundle explorer
	local_server: bool,
	#[arg(long, default_value_t = false)]
	/// Start the explorer server along with the client
	with_server: bool,
}

impl Command {
	fn run_client(&self) -> Result<()> {
		let guh = build::client::Command {
			debug: !self.release,
			local_server: self.local_server,
			sub_target: None,
			no_minify_ssr: false,
		};
		info!("Building client wasm");
		guh.build_wasm()?;
		if self.release {
			info!("Building client for preview");
			guh.build_vite()?;
			info!("Starting preview server");
			process::Command::npx("vite")?
				.arg("preview")
				.run()?;
		} else {
			info!("Starting vite dev server");
			process::Command::npx("vite")?
				.arg("dev")
				.run()?;
			info!("Vite dev server stopped");
		}
		Ok(())
	}

	fn run_server(&self) -> Result<()> {
		info!("running server");
		crate::run::server::Command {
			debug: !self.release,
			clean_cache: false,
		}
		.run()
	}
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		thread::scope(|s| {
			if self.local_server {
				// SAFETY: we're have a single-thread here
				unsafe { env::set_var("IS_SERVER_LOCAL", "1") };
			}
			let server_handle = self
				.with_server
				.then(|| s.spawn(|| self.run_server()));
			let client_handle = s.spawn(|| self.run_client());
			client_handle
				.join()
				.expect("client thread panicked")?;
			if let Some(server_handle) = server_handle {
				server_handle
					.join()
					.expect("server thread panicked")?;
			}
			Ok(())
		})
	}
}
