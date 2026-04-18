use anyhow::{Context, Result, anyhow, bail};
use clap::Args;
use itertools::Itertools as _;
use oxc::{allocator::Allocator, ast::ast::RegExpFlags};
use regress::escape;
use std::{
	env,
	fs,
	path::{Path, PathBuf},
};
use tokio::task;
use tracing::{debug, trace, warn};
use vencord_ast_parser::{Match, MatchRegex, Patch, VencordAstParser};

use crate::util::Stage;

#[derive(Args, Debug, Clone)]
pub struct VencordOpts {
	/// Path to vencord source dir. Defaults to $PWD
	#[arg(short = 'C', long, default_value_os_t = default_vencord_dir())]
	pub vencord_dir: PathBuf,
	/// Dirs to load plugins from, relative to the root dir.
	///
	/// If not passed, will be the result of globing [`vencord_dir`](Self::vencord_dir) with `src/*plugins/{., _api, _core}`
	///
	/// note: `src/userplugins` is not globbed by default and needs to be explicitly passed
	#[arg(short, long)]
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
	mut opts: VencordOpts,
	bar: Stage,
) -> Result<Vec<Plugin>> {
	task::spawn_blocking(move || do_collect_patches(&mut opts, bar)).await?
}

#[expect(clippy::needless_pass_by_value, reason = "RAII")]
fn do_collect_patches(
	opts: &mut VencordOpts,
	bar: Stage,
) -> Result<Vec<Plugin>> {
	bar.msg("Globbing plugins");
	if opts.plugin_dirs.is_empty() {
		opts.plugin_dirs = infer_plugin_dirs(&opts.vencord_dir)
			.context("infer plugin dirs")?;
		debug!("Inferred plugin dirs: {:?}", opts.plugin_dirs);
	}
	let mut plugins = Vec::new();
	for plugin_base_dir in opts
		.plugin_dirs
		.iter()
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
		let path = p.entry_point.to_string_lossy();
		let parser =
			VencordAstParser::try_new(&allocator, &p.entry_source, Some(&path))
				.map_err(|e| anyhow!(e))?;
		p.patches = parser.patches()?;
		allocator.reset();
	}

	bar.msg("Binding plugin IDs");
	bind_plugin_ids(&mut plugins);
	bar.msg("Compiling plugin regexes");
	compile_plugin_regexes(&mut plugins);

	Ok(plugins)
}

pub async fn collect_plugins_from_paths(
	paths: Vec<(PathBuf, String)>,
) -> Result<Vec<Plugin>> {
	task::spawn_blocking(move || do_collect_plugins_from_paths(paths)).await?
}

fn do_collect_plugins_from_paths(
	paths: Vec<(PathBuf, String)>,
) -> Result<Vec<Plugin>> {
	let mut allocator = Allocator::new();
	let mut plugins = paths
		.into_iter()
		.map(|(entry_point, entry_source)| Plugin {
			entry_point,
			entry_source,
			patches: Vec::new(),
		})
		.collect_vec();
	for plugin in &mut plugins {
		let path = plugin.entry_point.to_string_lossy();
		let parser = VencordAstParser::try_new(
			&allocator,
			&plugin.entry_source,
			Some(&path),
		)
		.map_err(|e| anyhow!(e))?;
		plugin.patches = parser.patches()?;
		allocator.reset();
	}
	drop(allocator);
	debug!("Binding plugin IDs");
	bind_plugin_ids(&mut plugins);
	debug!("Compiling plugin regexes");
	compile_plugin_regexes(&mut plugins);
	Ok(plugins)
}

pub fn infer_plugin_dirs(vencord_dir: &Path) -> Result<Vec<PathBuf>> {
	const PLUGIN_SUB_DIRS: &[&str] = &["_core", "_api"];
	let vencord_dir = vencord_dir
		.canonicalize()
		.context("Failed to canonicalize vencord dir")?;
	let src_dir = vencord_dir.join("src");
	let mut ret = Vec::<PathBuf>::new();
	for path in fs::read_dir(&src_dir).with_context(|| {
		anyhow!("failed to read src dir: {}", src_dir.as_path().display())
	})? {
		let path = path?;
		if !path.file_type()?.is_dir() {
			continue;
		}
		let path = path.path().canonicalize()?;
		// This should never be none as read_dir does not iterate over . and .. entries
		let file_name = path
			.file_name()
			.unwrap()
			.to_string_lossy();
		if file_name == "userplugins" {
			continue;
		}
		if file_name.ends_with("plugins") {
			for sub_dir in PLUGIN_SUB_DIRS {
				let Ok(sub_dir) = path.join(sub_dir).canonicalize() else {
					continue;
				};
				let sub_dir = sub_dir
					.strip_prefix(&vencord_dir)
					.context("inferred plugin dir is not in vencord dir")?
					.to_owned();
				if sub_dir.is_dir() {
					ret.push(sub_dir);
				}
			}
			let path = path
				.strip_prefix(&vencord_dir)
				.context("inferred plugin dir is not in vencord dir")?
				.to_owned();
			ret.push(path);
		}
	}
	Ok(ret)
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

fn default_vencord_dir() -> PathBuf {
	env::current_dir().expect("Failed to get current directory")
}

#[allow(clippy::cognitive_complexity)]
pub fn glob_plugins_for_dir(
	dir: &Path,
	plugins: &mut Vec<Plugin>,
) -> Result<()> {
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
