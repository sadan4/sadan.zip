mod hash;
mod parser;
use anyhow::{Result, bail};
use clap::Args;
use derive_more::{Eq, PartialEq};
use oxc::{allocator::Allocator, ast::ast::{RegExp, RegExpFlags}, span::Span};
use regress::{Flags, Regex};
use std::{
    cell::OnceCell, env, fs::{self, ReadDir}, hash::{Hash, Hasher}, path::{Path, PathBuf}, time::Instant
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

pub async fn collect_patches(opts: VencordOpts) -> Result<Vec<Plugin>> {
    tokio::task::spawn_blocking(move || do_collect_patches(opts)).await?
}

fn do_collect_patches(opts: VencordOpts) -> Result<Vec<Plugin>> {
    let mut plugins = Vec::new();
    for plugin_base_dir in opts
        .plugin_dirs
        .into_iter()
        .map(|d| opts.vencord_dir.join(d))
    {
        if !plugin_base_dir.exists() {
            bail!(
                "plugin base dir {} doesn't exist",
                plugin_base_dir.display()
            );
        }
        glob_plugins_for_dir(&plugin_base_dir, &mut plugins)?;
    }

    info!("Found {} plugins, parsing...", plugins.len());

    let start = Instant::now();
    let mut allocator = Allocator::new();
    for p in &mut plugins {
        debug_assert!(
            p.patches.is_empty(),
            "Patches should be empty before parsing"
        );
        p.patches = parse_patches(&allocator, &p.entry_point)?;
        allocator.reset();
    }

    bind_plugin_ids(&mut plugins);

    info!("Collecting patches took {:.2?}", start.elapsed());

    Ok(plugins)
}

fn bind_plugin_ids(plugins: &mut [Plugin]) {
    for (id, plugin) in plugins.iter_mut().enumerate() {
        for patch in &mut plugin.patches {
            patch.plugin_id = Some(id as u16);
        }
    }
}

// TODO: pack all/no_warn into flags?

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Patch {
    pub plugin_id: Option<u16>,
    pub all: bool,
    pub no_warn: bool,
    pub find: MatchLike,
    pub replacement: Vec<Replacement>,
}

impl Patch {
    pub fn plugin_id(&self) -> u16 {
        self.plugin_id.expect("Plugin ID not set")
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Replacement {
    pub match_: MatchLike,
    pub replace: ReplaceLike,
    pub no_warn: bool,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ReplaceLike {
    pub v: Replacer,
    pub s: Span,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Replacer {
    Str(String),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MatchLike {
    pub v: Match,
    pub s: Span,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Match {
    Str(String),
    Regex(MatchRegex),
}

#[derive(Debug, PartialEq, Eq)]
pub struct MatchRegex {
    pub pattern: String,
    pub flags: RegExpFlags,
    #[eq(skip)]
    pub regex: OnceCell<Result<Regex>>,
}

impl Hash for MatchRegex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pattern.hash(state);
        self.flags.hash(state);
    }
}

impl MatchRegex {
    pub fn regex(&self) -> &Result<Regex> {
        self.regex.get_or_init(|| {
            let f = |f| self.flags.contains(f);
            Ok(Regex::with_flags(&self.pattern, Flags {
                icase: f(RegExpFlags::I),
                multiline: f(RegExpFlags::M),
                dot_all: f(RegExpFlags::S),
                unicode: f(RegExpFlags::U),
                unicode_sets: f(RegExpFlags::V),
                no_opt: false,
            })?)
        })
    }
}

#[derive(Debug)]
pub struct Plugin {
    pub entry_point: PathBuf,
    pub patches: Vec<Patch>,
}

impl Plugin {
    const fn new(entry_point: PathBuf) -> Self {
        Self {
            entry_point,
            patches: Vec::new(),
        }
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
            Plugin::new(entry_point)
        } else {
            Plugin::new(file_name)
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
