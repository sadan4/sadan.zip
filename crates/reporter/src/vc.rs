mod parser;
use anyhow::{Result, bail};
use clap::Args;
use oxc::allocator::Allocator;
use std::{
    env,
    fs::ReadDir,
    path::{Path, PathBuf},
};
use tokio::fs;
use tokio_stream::{StreamExt as _, wrappers::ReadDirStream};
use tracing::{trace, warn};

fn default_plugin_dirs() -> impl IntoIterator<Item = PathBuf> {
    ["src/plugins", "src/plugins/_api", "src/plugins/_core"]
        .iter()
        .map(PathBuf::from)
}

fn default_vencord_dir() -> PathBuf {
    env::current_dir().expect("Failed to get current directory")
}

#[derive(Args)]
pub struct VencordOpts {
    /// Path to vencord source dir. Defaults to $PWD
    #[arg(short = 'C', long, default_value_os_t = default_vencord_dir())]
    pub vencord_dir: PathBuf,
    /// Dirs to load plugins from, relative to the root dir.
    #[arg(short, long, default_values_os_t = default_plugin_dirs())]
    pub plugin_dirs: Vec<PathBuf>,
}

pub async fn collect_patches(opts: VencordOpts) -> Result<Vec<StandalonePatch>> {
    let mut plugins = Vec::new();
    for plugin_base_dir in ["src/plugins", "src/plugins/_api", "src/plugins/_core"]
        .into_iter()
        .map(|d| opts.vencord_dir.join(d))
    {
        glob_plugins_for_dir(&plugin_base_dir, &mut plugins).await?;
    }
    dbg!(&plugins);
    bail!("TODO");
}

#[derive(Debug)]
pub struct StandalonePatch {}

#[derive(Debug)]
struct Patch {}
#[derive(Debug)]
struct Plugin {
    entry_point: PathBuf,
    patches: Vec<Patch>,
}

impl Plugin {
    fn try_new(entry_point: PathBuf) -> Result<Self> {
        Ok(Self {
            entry_point,
            patches: Vec::new(),
        })
    }
}

async fn glob_plugins_for_dir(dir: &Path, plugins: &mut Vec<Plugin>) -> Result<()> {
    let files = fs::read_dir(dir).await?;
    let mut st = ReadDirStream::new(files);
    while let Some(path) = st.next().await {
        let path = path?;
        let file_name = path.path();

        if let Some(stem) = file_name.file_stem() {
            if stem == "index" {
                trace!(?file_name, "ignoring root index file in plugin dir");
                continue;
            }
            if stem.to_string_lossy().starts_with('_') {
                trace!(?file_name, "ignoring path starting with `_`")
            }
        }
        let plugin = if path.file_type().await?.is_dir() {
            let Some(entry_point) = resolve_plugin_entry_point(&file_name) else {
                warn!(
                    plugin_dir = ?file_name,
                    "Failed to resolve entry point for plugin, skipping"
                );
                continue;
            };
            Plugin::try_new(entry_point)?
        } else {
            Plugin::try_new(file_name)?
        };

        plugins.push(plugin);
    }

    Ok(())
}

fn resolve_plugin_entry_point(plugin_dir: &Path) -> Option<PathBuf> {
    const FILES: &[&str] = &["index.ts", "index.tsx", "index.js", "index.jsx"];

    for file in FILES {
        let entry_point = plugin_dir.join(file);
        // TODO: use async exists
        if entry_point.exists() {
            trace!(?plugin_dir, ?entry_point, "Resolved entry point for plugin");
            return Some(entry_point);
        }
    }

    None
}
