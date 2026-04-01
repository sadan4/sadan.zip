use anyhow::{Result, bail};
use clap::Args;
use oxc::{allocator::Allocator, ast::ast::RegExpFlags};
use regress::escape;
use std::{
	env,
	fs,
	path::{Path, PathBuf},
};
use tracing::{trace, warn};
use vencord_ast_parser::{Match, MatchRegex, Patch, VencordAstParser};

use crate::util::Stage;

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

pub async fn collect_patches(
	opts: VencordOpts,
	bar: Stage,
) -> Result<Vec<Plugin>> {
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

fn default_plugin_dirs() -> impl IntoIterator<Item = PathBuf> {
	["src/plugins", "src/plugins/_api", "src/plugins/_core"]
		.iter()
		.map(PathBuf::from)
}

fn default_vencord_dir() -> PathBuf {
	env::current_dir().expect("Failed to get current directory")
}

#[allow(clippy::cognitive_complexity)]
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
			let Some(entry_point) = resolve_plugin_entry_point(&file_name)
			else {
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
			trace!(
				?plugin_dir,
				?entry_point,
				"Resolved entry point for plugin"
			);
			return Some(entry_point);
		}
	}

	None
}
