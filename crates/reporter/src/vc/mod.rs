mod hash;
mod parser;
use anyhow::{Result, bail};
use clap::Args;
use derive_more::{Eq, PartialEq};
use itertools::Itertools;
use memchr::memmem::Finder;
use oxc::{allocator::Allocator, ast::ast::RegExpFlags, span::Span};
use regress::{Flags, Regex, escape};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use tracing::{trace, warn};

use crate::{util::Stage, vc::parser::vencord_ast_parser::VencordAstParser};

#[derive(Args, Debug)]
pub struct VencordOpts {
    /// Path to vencord source dir. Defaults to $PWD
    #[arg(short = 'C', long, default_value_os_t = default_vencord_dir())]
    pub vencord_dir: PathBuf,
    /// Dirs to load plugins from, relative to the root dir.
    #[arg(short, long, default_values_os_t = default_plugin_dirs())]
    pub plugin_dirs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Plugin {
    pub entry_point: PathBuf,
    pub entry_source: String,
    pub patches: Vec<Patch>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Patch {
    pub plugin_id: Option<u16>,
    pub all: bool,
    pub no_warn: bool,
    pub find: MatchLike,
    pub replacement: Vec<Replacement>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Replacement {
    pub match_: MatchLike,
    pub replace: ReplaceLike,
    pub no_warn: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ReplaceLike {
    pub v: Replacer,
    pub s: Span,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Replacer {
    Str(String),
    Template(TemplateEvaluator),
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct TemplateEvaluator {
    lits: Vec<String>,
    captures: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct MatchLike {
    pub v: Match,
    pub s: Span,
}

#[derive(Debug, Serialize)]
pub enum Match {
    #[serde(with = "FinderDef")]
    Str(Finder<'static>),
    Regex(MatchRegex),
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct MatchRegex {
    pub pattern: String,
    #[serde(with = "RegExpFlagsDef")]
    pub flags: RegExpFlags,
    #[serde(skip)]
    #[eq(skip)]
    pub regex: Option<Result<Regex>>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "RegExpFlags")]
struct RegExpFlagsDef {
    #[serde(getter = "RegExpFlags::bits")]
    bits: <oxc::ast::ast::RegExpFlags as bitflags::Flags>::Bits,
}

#[derive(Serialize)]
#[serde(remote = "Finder")]
struct FinderDef {
    #[serde(getter = "finder_get_needle")]
    needle: Box<str>,
}

impl TemplateEvaluator {
    pub fn make_replacer<'a: 'b, 'b: 'a>(
        &'a self,
        src: &'b str,
    ) -> impl 'a + 'b + Fn(&regress::Match) -> String {
        |m| {
            let lits = self.lits.iter().map(String::as_str);
            let caps = self.captures.iter().map(|&i| {
                // FIXME: catch this with a lint while parsing the patch in the first place
                let range = m.group(i as _).expect("capture group out of range");
                &src[range]
            });
            // we always assert when we construct Self, this is for sanity
            debug_assert_eq!(self.lits.len(), self.captures.len() + 1);
            lits.interleave_shortest(caps).collect()
        }
    }
}

impl Patch {
    pub const fn plugin_id(&self) -> u16 {
        self.plugin_id.expect("Plugin ID not set")
    }
}

impl From<FinderDef> for Finder<'static> {
    fn from(value: FinderDef) -> Self {
        Finder::new(value.needle.as_bytes()).into_owned()
    }
}

#[expect(clippy::fallible_impl_from, reason = "TODO")]
impl From<RegExpFlagsDef> for RegExpFlags {
    fn from(value: RegExpFlagsDef) -> Self {
        Self::from_bits(value.bits).unwrap()
    }
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
    pub const fn regex(&self) -> &Result<Regex> {
        self.regex.as_ref().expect("Regex not compiled")
    }
}

impl Match {
    #[must_use]
    pub const fn as_regex(&self) -> Option<&MatchRegex> {
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

impl Eq for Match {}

impl Plugin {
    fn try_new(entry_point: &Path) -> Result<Self> {
        let entry_point = entry_point.canonicalize()?;
        Ok(Self {
            entry_source: fs::read_to_string(&entry_point)?,
            entry_point,
            patches: Vec::new(),
        })
    }
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
        let parser = VencordAstParser::try_new(&allocator, &p.entry_source)?;
        p.patches = parser.patches()?;
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

fn finder_get_needle(finder: &Finder<'_>) -> Box<str> {
    str::from_utf8(finder.needle())
        .expect("finder is not a utf8 string")
        .into()
}

fn default_plugin_dirs() -> impl IntoIterator<Item = PathBuf> {
    ["src/plugins", "src/plugins/_api", "src/plugins/_core"]
        .iter()
        .map(PathBuf::from)
}

fn default_vencord_dir() -> PathBuf {
    env::current_dir().expect("Failed to get current directory")
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
            Plugin::try_new(&entry_point)?
        } else {
            Plugin::try_new(&file_name)?
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
