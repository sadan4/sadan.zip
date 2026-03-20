use std::{path::Path, process};

use crate::{
    Runnable, clean,
    util::{cmd::CommandExt as _, fs, server::ServerTarget},
};
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use tracing::{info, instrument};

#[derive(Args, Debug)]
pub struct Command {
    #[arg(short, long, default_value_t = false)]
    /// Build in release mode with optimizations.
    pub release: bool,
    #[arg(short = 'o', long, default_value_t = false)]
    /// Do not build any of the dependencies of [`Self::target`]
    pub no_deps: bool,
    #[command(flatten)]
    pub js_mode: ArgJsMode,
    #[arg(value_enum, default_value_t = ServerTarget::Native)]
    /// The sub-section of the server to build
    /// Unless [`Self::no_deps`] is set, this will also build any dependencies of the sub-section.
    pub target: ServerTarget,
}

#[derive(Args, Debug, Copy, Clone)]
pub struct ArgJsMode {
    #[arg(short, long, value_enum, default_value_t)]
    /// How to build the js code for the server.
    pub js_mode: JsMode,
}

#[derive(Default, ValueEnum, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum JsMode {
    #[default]
    /// Build the js code to a normal js file that can be ran by node
    Bundler,
    /// Build the js code to a standalone executable with bun. 
    /// The executable will then be embed in the final binary and written to disk at runtime.
    Binary,
}

impl JsMode {
    /// Returns `true` if the js mode is [`Binary`].
    ///
    /// [`Binary`]: JsMode::Binary
    #[must_use]
    pub const fn is_binary(&self) -> bool {
        matches!(self, Self::Binary)
    }
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
        let is_exe = self.js_mode.js_mode.is_binary();
        process::Command::new("bun")
            .arg("build")
            .arg("server/parser-worker.ts")
            .arg_if(!is_exe, "--outdir")
            .arg_if(!is_exe, "dist.server")
            .arg_if(is_exe, "--outfile")
            .arg_if(is_exe, "dist.server/parser-worker")
            .arg_if(!is_exe, "--target")
            .arg_if(!is_exe, "node")
            .arg_if(is_exe, "--compile")
            .arg_if(is_exe && self.release, "--bytecode")
            .arg_if(self.release, "--minify")
            .arg_if(!self.release, "--sourcemap")
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
        let is_exe = self.js_mode.js_mode.is_binary();
        process::Command::cargo(cargo_subcmd)?
            .arg("-p")
            .arg("explorer_server")
            .arg_if(is_exe, "--features")
            .arg_if(is_exe, "js-bin")
            .arg_if(self.release, "--release")
            .run()
    }
}

impl Runnable for Command {
    #[instrument(skip(self))]
    fn run(&self) -> Result<()> {
        info!(?self, "Building server");
        match self.target {
            ServerTarget::Napi => self.build_server_napi()?,
            ServerTarget::Js => self.build_server_js()?,
            ServerTarget::Native => self.build_server("build")?,
        }
        info!("Done");
        Ok(())
    }
}
