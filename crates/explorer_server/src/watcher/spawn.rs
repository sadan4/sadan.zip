use anyhow::{Context as _, Result};
use std::{io, process::Stdio};
use tokio::process;

pub trait BuildParserWorker {
    async fn setup() -> Result<()>;
    async fn spawn(data: io::PipeReader) -> Result<()>;
}

#[allow(dead_code)]
const JS_PATH: &str = match option_env!("JS_PATH") {
    Some(path) => path,
    None => "dist.server/parser-worker.js",
};

async fn do_spawn(data: io::PipeReader, cmd: &mut process::Command) -> Result<()> {
    match cmd
        .stdin(data)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
    {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(anyhow::anyhow!("js process failed with status {s}")),
        Err(e) => Err(e).context("Failed to spawn js handler"),
    }
}

#[expect(dead_code)]
pub struct BunSpawner;

#[cfg_attr(feature = "js-bin", expect(dead_code))]
pub struct NodeSpawner;

impl BuildParserWorker for BunSpawner {
    async fn setup() -> Result<()> {
        Ok(())
    }

    async fn spawn(data: io::PipeReader) -> Result<()> {
        do_spawn(data, process::Command::new("bun").arg(JS_PATH)).await
    }
}

impl BuildParserWorker for NodeSpawner {
    async fn setup() -> Result<()> {
        Ok(())
    }

    async fn spawn(data: io::PipeReader) -> Result<()> {
        do_spawn(data, process::Command::new("node").arg(JS_PATH)).await
    }
}

// #[cfg(feature = "js-bin")]
pub mod bin {
    use std::{env, path::PathBuf, sync::LazyLock};

    use const_format::formatc;
    use tokio::fs;

    use crate::{BIN_EXT, BUILD_SEED};

    use super::*;

    pub struct BinarySpawner;

    static BIN_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
        const BIN_NAME: &str = formatc!("parser-worker-{BUILD_SEED}{BIN_EXT}");
        let mut path = env::temp_dir();
        path.push(BIN_NAME);
        path
    });

    impl BuildParserWorker for BinarySpawner {
        async fn setup() -> Result<()> {
            #[cfg(windows)]
            const BIN_DATA: &[u8] = include_bytes!("../../../../dist.server/parser-worker.exe");
            #[cfg(not(windows))]
            const BIN_DATA: &[u8] = include_bytes!("../../../../dist.server/parser-worker");
            if BIN_PATH.exists() {
                return Ok(());
            }
            fs::write(&*BIN_PATH, BIN_DATA).await?;
            Ok(())
        }

        async fn spawn(data: io::PipeReader) -> Result<()> {
            do_spawn(data, &mut process::Command::new(&*BIN_PATH)).await
        }
    }
}

#[cfg(feature = "js-bin")]
pub use bin::BinarySpawner as DefaultBuildParserWorker;

#[cfg(not(feature = "js-bin"))]
pub use NodeSpawner as DefaultBuildParserWorker;
