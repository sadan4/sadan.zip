use std::{path::Path, process};

use crate::{
    Runnable, clean,
    util::{cmd::CommandExt as _, fs, server::ServerTarget},
};
use anyhow::{Context, Result};
use clap::Args;
use tracing::{info, instrument};

#[derive(Args, Debug)]
pub struct Command {
    #[arg(long, default_value_t = false)]
    pub(crate) release: bool,
    #[arg(short = 'o', long, default_value_t = false)]
    /// Do not build any of the dependencies of [`Self::target`]
    pub(crate) no_deps: bool,
    #[arg(value_enum, default_value_t = ServerTarget::Native)]
    pub(crate) target: ServerTarget,
}

impl Command {
    #[instrument(skip(self))]
    fn build_server_napi(&self) -> Result<()> {
        info!("Building server napi module");
        process::Command::npx("napi")?
            .arg("build")
            .arg_if(self.release, "--release")
            .arg("-p")
            .arg("explorer_writer")
            .arg("-o")
            .arg(Path::new("server").join("native"))
            .run()?;
        info!("Renaming napi d.ts file");
        // move the d.ts file so that it ends with .node.d.ts
        fs::rename("server/native/index.d.ts", "server/native/index.node.d.ts")
            .context("Failed to rename napi d.ts")?;
        Ok(())
    }
    #[instrument(skip(self))]
    fn build_server_js(&self) -> Result<()> {
        if !self.no_deps {
            self.build_server_napi()?;
        }
        info!("cleaning js output folder");
        clean::server::Command {
            leave_dts: true,
            target: vec![ServerTarget::Js],
        }
        .run()
        .with_context(|| "failed to clean js output")?;
        info!("Building server js code");
        process::Command::new("bun")
            .arg("build")
            .arg("server/index.ts")
            .arg("--outdir")
            .arg("dist.server")
            .arg("--target")
            .arg("node")
            .arg_if(self.release, "--minify")
            .run()
        // process::Command::npx("rollup")?
        //     .arg("-c")
        //     .arg(Path::new("server").join("rollup.config.ts"))
        //     .run()
    }
    #[instrument(skip(self))]
    pub fn build_server(&self, cargo_subcmd: &str) -> Result<()> {
        if !self.no_deps {
            self.build_server_js()?;
        }
        info!("Building server native code");
        process::Command::cargo(cargo_subcmd)?
            .arg("-p")
            .arg("explorer_server")
            .arg_if(self.release, "--release")
            .run()
    }
}

impl Runnable for Command {
    #[instrument]
    fn run(&self) -> Result<()> {
        info!("Building server");
        match self.target {
            ServerTarget::Napi => self.build_server_napi()?,
            ServerTarget::Js => self.build_server_js()?,
            ServerTarget::Native => self.build_server("build")?,
        }
        info!("Done");
        Ok(())
    }
}
