use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

pub mod client;
pub mod server;

#[derive(Args)]
pub struct Command {
    #[command(subcommand)]
    target: Target,
}

impl Runnable for Command {
    fn run(&self) -> Result<()> {
        match &self.target {
            Target::Server(c) => c.run(),
            Target::Client(c) => c.run(),
        }
    }
}

#[derive(Subcommand)]
enum Target {
    Server(server::Command),
    Client(client::Command),
}
