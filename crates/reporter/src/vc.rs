mod parser;
mod hash;
use anyhow::{Result, bail};
use clap::Args;
use oxc::{allocator::Allocator, ast::ast::RegExpFlags, span::Span};
use std::{
    env,
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};
use tokio_stream::{StreamExt as _, wrappers::ReadDirStream};
use tracing::{info, trace, warn};

use crate::vc::parser::parse_patches;

fn default_plugin_dirs() -> impl IntoIterator<Item = PathBuf> {
    ["src/plugins", "src/plugins/_api", "src/plugins/_core"]
        .iter()
        .map(PathBuf::from)
}

fn default_vencord_dir() -> PathBuf {
    env::current_dir().expect("Failed to get current directory")
}

#[derive(Args, Debug)]
pub struct VencordOpts {
    /// Path to vencord source dir. Defaults to $PWD
    #[arg(short = 'C', long, default_value_os_t = default_vencord_dir())]
    pub vencord_dir: PathBuf,
    /// Dirs to load plugins from, relative to the root dir.
    #[arg(short, long, default_values_os_t = default_plugin_dirs())]
    pub plugin_dirs: Vec<PathBuf>,
}

pub async fn collect_patches(opts: VencordOpts) -> Result<Vec<StandalonePatch>> {
    tokio::task::spawn_blocking(move || do_collect_patches(opts)).await?
}

fn do_collect_patches(opts: VencordOpts) -> Result<Vec<StandalonePatch>> {
    let mut plugins = Vec::new();
    for plugin_base_dir in opts
        .plugin_dirs
        .into_iter()
        .map(|d| opts.vencord_dir.join(d))
    {
        if !plugin_base_dir.exists() {
            bail!("plugin base dir {} doesn't exist", plugin_base_dir.display());
        }
        glob_plugins_for_dir(&plugin_base_dir, &mut plugins)?;
    }
    info!("Found {} plugins, parsing...", plugins.len());
    let allocator = Allocator::new();
    for p in plugins {
        parse_patches(&allocator, &p.entry_point)?;
    }
    bail!("TODO");
}

#[derive(Debug)]
pub struct StandalonePatch {}

#[derive(Debug)]
struct Patch {
    all: bool,
    no_warn: bool,
    find: MatchLike,
    replacement: Vec<Replacement>,
}

#[derive(Debug)]
struct Replacement {
    match_: MatchLike,
    replace: Replacer,
    no_warn: bool,
}

#[derive(Debug)]
enum Replacer {
    Str(String),
}

#[derive(Debug)]
struct MatchLike {
    v: Match,
    s: Span,
}

#[derive(Debug)]
enum Match {
    Str(String),
    Regex(String, RegExpFlags),
}

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

fn glob_plugins_for_dir(dir: &Path, plugins: &mut Vec<Plugin>) -> Result<()> {
    for path in fs::read_dir(dir)? {
        let path = path?;
        let file_name = path.path();

        if let Some(stem) = file_name.file_stem() {
            if stem == "index" {
                trace!(?file_name, "ignoring root index file in plugin dir");
                continue;
            }
            if stem.to_string_lossy().starts_with('_') {
                trace!(?file_name, "ignoring path starting with `_`");
                continue;
            }
        }
        let plugin = if path.file_type()?.is_dir() {
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
