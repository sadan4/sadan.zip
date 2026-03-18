use crate::Runnable;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct Command {
    #[command(subcommand)]
    target: Target,
}

impl Runnable for Command {
    fn run(&self) -> anyhow::Result<()> {
        match &self.target {
            Target::Server(c) => c.run(),
            Target::BuildCache(c) => c.run(),
        }
    }
}

#[derive(Subcommand)]
pub enum Target {
    Server(server::Command),
    BuildCache(build_cache::Command),
}

pub mod build_cache {
    use anyhow::Result;
    use clap::Args;
    use tracing::info;

    use crate::{Runnable, util::fs};

    #[derive(Args, Debug)]
    pub struct Command;

    impl Runnable for Command {
        fn run(&self) -> Result<()> {
            info!("cleaing build cache");
            // TODO: show size of cache cleared
            fs::rm_rf_if_exists("builds")?;
            info!("build cache cleared");
            Ok(())
        }
    }
}

pub mod server {
    use std::process;

    use anyhow::Result;
    use clap::Args;
    use tracing::{info, instrument};

    use crate::{
        Runnable,
        util::{cmd::CommandExt as _, fs, server::ServerTarget},
    };

    #[derive(Args, Debug)]
    pub struct Command {
        #[arg(long, default_value_t = false)]
        pub leave_dts: bool,
        #[arg(value_enum, default_values_t = vec![ServerTarget::Napi, ServerTarget::Js, ServerTarget::Native])]
        pub target: Vec<ServerTarget>,
    }

    impl Runnable for Command {
        #[instrument]
        fn run(&self) -> Result<()> {
            for target in &self.target {
                info!("Cleaning server target: {target:?}");
                match target {
                    ServerTarget::Napi => {
                        fs::rm_if_exists("server/native/index.node")?;
                        if !self.leave_dts {
                            fs::rm_if_exists("server/native/index.d.ts")?;
                        }
                    }
                    ServerTarget::Js => {
                        fs::rm_rf_if_exists("dist.server")?;
                    }
                    ServerTarget::Native => {
                        process::Command::cargo("clean")?
                            .arg("-p")
                            .arg("explorer_server")
                            .run()?;
                    }
                }
            }
            Ok(())
        }
    }
}
