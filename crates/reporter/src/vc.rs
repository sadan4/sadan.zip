mod hash;
mod parser;
use anyhow::{Result, bail};
use clap::Args;
use derive_more::{Eq, IsVariant, PartialEq, TryInto, TryUnwrap, Unwrap};
use memchr::memmem::Finder;
use oxc::{allocator::Allocator, ast::ast::RegExpFlags, span::Span};
use regress::{Flags, Regex, escape};
use std::{
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::Instant,
};
use tokio_stream::{StreamExt as _, wrappers::ReadDirStream};
use tracing::{info, trace, warn};

use crate::{util::Stage, vc::parser::parse_patches};

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

pub async fn collect_patches(opts: VencordOpts, bar: Stage) -> Result<Vec<Plugin>> {
    tokio::task::spawn_blocking(move || do_collect_patches(opts, bar)).await?
}

#[expect(clippy::needless_pass_by_value, reason = "RAII")]
fn do_collect_patches(opts: VencordOpts, bar: Stage) -> Result<Vec<Plugin>> {
    bar.msg("Globbing plugins");
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

    bar.msg(format!("Parsing {} plugins", plugins.len()));
    let mut allocator = Allocator::new();
    for p in &mut plugins {
        debug_assert!(
            p.patches.is_empty(),
            "Patches should be empty before parsing"
        );
        p.patches = parse_patches(&allocator, p, &bar)?;
        allocator.reset();
    }

    bar.msg("Binding plugin IDs");
    bind_plugin_ids(&mut plugins);
    bar.msg("Compiling plugin regexes");
    compile_plugin_regexes(&mut plugins);

    Ok(plugins)
}

fn bind_plugin_ids(plugins: &mut [Plugin]) {
    for (id, plugin) in plugins.iter_mut().enumerate() {
        for patch in &mut plugin.patches {
            patch.plugin_id = Some(id as u16);
        }
    }
}

fn compile_plugin_regexes(plugins: &mut [Plugin]) {
    for plugin in plugins {
        for patch in &mut plugin.patches {
            if let Match::Regex(r) = &mut patch.find.v {
                r.make_regex();
            }
            for replacement in &mut patch.replacement {
                // transform it to a regex here so we can cache it easier.
                if let Match::Str(s) = &replacement.match_.v {
                    let regex = MatchRegex {
                        // we only ever create a finder with a utf8 string
                        // so this should never error
                        pattern: escape(str::from_utf8(s.needle()).unwrap()),
                        flags: RegExpFlags::empty(),
                        regex: None,
                    };
                    replacement.match_.v = Match::Regex(regex);
                }
                match &mut replacement.match_.v {
                    Match::Regex(r) => {
                        r.make_regex();
                        // the match is a string, we need to compile it to a regex
                    }
                    Match::Str(_) => {
                        // we just set any potential Match::Str to Match::Regex above
                        unreachable!()
                    }
                }
            }
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

#[derive(Debug)]
pub enum Match {
    Str(Finder<'static>),
    Regex(MatchRegex),
}

impl Match {
    #[must_use]
    pub fn as_regex(&self) -> Option<&MatchRegex> {
        if let Self::Regex(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Hash for Match {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Str(s) => s.needle().hash(state),
            Self::Regex(s) => s.hash(state),
        }
    }
}

impl PartialEq for Match {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Str(l0), Self::Str(r0)) => l0.needle() == r0.needle(),
            (Self::Regex(l0), Self::Regex(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for Match {

}

#[derive(Debug, PartialEq, Eq)]
pub struct MatchRegex {
    pub pattern: String,
    pub flags: RegExpFlags,
    #[eq(skip)]
    pub regex: Option<Result<Regex>>,
}

impl Hash for MatchRegex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pattern.hash(state);
        self.flags.hash(state);
    }
}

impl MatchRegex {
    pub fn make_regex(&mut self) {
        if self.regex.is_some() {
            return;
        }
        let f = |f| self.flags.contains(f);
        self.regex = Some(
            Regex::with_flags(
                &self.pattern,
                Flags {
                    icase: f(RegExpFlags::I),
                    multiline: f(RegExpFlags::M),
                    dot_all: f(RegExpFlags::S),
                    unicode: f(RegExpFlags::U),
                    unicode_sets: f(RegExpFlags::V),
                    no_opt: false,
                },
            )
            .map_err(Into::into),
        );
    }
    pub fn regex(&self) -> &Result<Regex> {
        self.regex.as_ref().expect("Regex not compiled")
    }
}

#[derive(Debug)]
pub struct Plugin {
    pub entry_point: PathBuf,
    pub entry_source: String,
    pub patches: Vec<Patch>,
}

impl Plugin {
    fn try_new(entry_point: PathBuf) -> Result<Self> {
        let entry_point = entry_point.canonicalize()?;
        Ok(Self {
            entry_source: fs::read_to_string(&entry_point)?,
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
