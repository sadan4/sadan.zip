use crate::Runnable;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct Command {
    #[arg(long, default_value_t = false)]
    release: bool,
}

impl Runnable for Command {
    fn run(&self) -> Result<()> {
        todo!();
    }
}
