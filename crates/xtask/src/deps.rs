use std::process;

use anyhow::{Context, Result};

use crate::util::cmd::CommandExt;

pub fn pnpm_i() -> Result<()> {
    process::Command::new("pnpm")
        .arg("install")
        .run()
        .with_context(|| "Failed to install run pnpm install")
}
