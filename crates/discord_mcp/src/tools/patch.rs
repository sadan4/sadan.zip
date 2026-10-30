//! Testing plugin patches and finds against a build

use std::{
	collections::{BTreeMap, HashSet},
	fmt::Write as _,
	path::PathBuf,
	sync::Arc,
};

use anyhow::{Context as _, Result, anyhow, bail};
use discord_scraper::ScrapeProgress;
use explorer_types::{Channel, ModuleId};
use miette::{Diagnostic as _, NamedSource, Severity};
use oxc::allocator::Allocator;
use patch_engine::{
	ApplyOptions,
	SyntaxErrorReport,
	apply_patch,
	matches_module,
};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use reporter::{
	SourceWrapper,
	diag::ReporterError,
	reporter::{Msg, report_broken_patches},
	vc::{Plugin, VencordOpts, collect_patches, collect_plugins_from_paths},
};
use term_progress::{MultiProgressWrapper, Stage};
use tokio::task;
use vencord_ast_parser::{
	FindArg,
	FindType,
	FindUse,
	Match,
	Patch,
	VencordAstParser,
};
use webpack_ast_parser::export_map::{ExportValue, RangeExportMap};

use super::{Matcher, id_list};
use crate::{
	store::LoadedBundle,
	text::{LineIndex, truncate, unified_diff},
};

/// Where the plugin under test comes from
pub struct PluginInput {
	pub path: Option<String>,
	pub source: Option<String>,
}

impl PluginInput {
	async fn read(self) -> Result<(PathBuf, String)> {
		match (self.path, self.source) {
			(path, Some(source)) => Ok((
				PathBuf::from(path.as_deref().unwrap_or("plugin.tsx")),
				source,
			)),
			(Some(path), None) => {
				let source = tokio::fs::read_to_string(&path)
					.await
					.with_context(|| format!("Failed to read {path}"))?;
				Ok((PathBuf::from(path), source))
			}
			(None, None) => bail!("Pass plugin_path or plugin_source"),
		}
	}
}

/// Renders a reporter diagnostic against the plugin source, uncoloured
fn render_error(e: ReporterError, plugins: &Arc<Vec<Plugin>>) -> String {
	let id = e.plugin_id();
	let path = plugins[id as usize]
		.entry_point
		.to_string_lossy()
		.into_owned();
	let source = SourceWrapper(plugins.clone(), id);
	let report = miette::Error::new(e).with_source_code(
		NamedSource::new(path, source).with_language("JavaScript"),
	);
	format!("{report:?}")
}

fn is_error(e: &ReporterError) -> bool {
	!e.is_no_warn()
		&& e.severity()
			.is_none_or(|s| s == Severity::Error)
}

fn plugin_name(source: &str, path: &str) -> String {
	let alloc = Allocator::default();
	VencordAstParser::try_new(&alloc, source, Some(path))
		.ok()
		.and_then(|p| p.plugin_info().ok())
		.map_or_else(|| String::from("MyPlugin"), |info| info.name.into())
}

fn patch_header(out: &mut String, i: usize, patch: &Patch, src: &str) {
	let lines = LineIndex::new(src);
	let find = src
		.get(patch.find.s.start as usize..patch.find.s.end as usize)
		.unwrap_or("?");
	writeln!(
		out,
		"\n## patch {i} (line {}), find: {find}",
		lines.line(patch.span.start as usize)
	)
	.unwrap();
}

struct PatchRun<'a> {
	bundle: &'a LoadedBundle,
	plugins: &'a Arc<Vec<Plugin>>,
	name: &'a str,
	max_modules: usize,
	context: usize,
	alloc: Allocator,
	out: String,
}

impl PatchRun<'_> {
	fn run(&mut self, i: usize, patch: &Patch) {
		let plugin = &self.plugins[patch.plugin_id() as usize];
		patch_header(&mut self.out, i, patch, &plugin.entry_source);
		if let Match::Regex(r) = &patch.find.v
			&& let Err(source) = r.regex()
		{
			let e = ReporterError::BadRegexSyntax {
				plugin_id: patch.plugin_id(),
				source: source.clone(),
				regex_span: patch.find.s.into(),
				expanded: format!("/{}/{}", r.pattern, r.flags),
			};
			self.out
				.push_str("FAIL: the find is not a valid regex\n");
			self.out
				.push_str(&render_error(e, self.plugins));
			return;
		}
		// a Matcher avoids regress's worst cases, see `Matcher::Regex`
		let matcher = match &patch.find.v {
			Match::Regex(r) => {
				Matcher::regex(&r.pattern, &r.flags.to_string()).ok()
			}
			Match::Str(_) => None,
		};
		let mut ids = self
			.bundle
			.modules
			.par_iter()
			.filter(|(_, src)| match &matcher {
				Some(m) => m.is_match(src),
				None => matches_module(src, patch).unwrap_or(false),
			})
			.map(|(&id, _)| id)
			.collect::<Vec<_>>();
		ids.sort_unstable();
		match ids.len() {
			0 => {
				self.out.push_str(if patch.no_warn {
					"FAIL (noWarn): the find matches no module\n"
				} else {
					"FAIL: the find matches no module. Use find_module and \
					 search_modules to find where the code moved.\n"
				});
				return;
			}
			1 => {}
			n if !patch.all => writeln!(
				self.out,
				"WARN: the find matches {n} modules ({}). Vencord only \
				 patches the first; make the find more specific or add \
				 `all: true`.",
				id_list(&ids, 10)
			)
			.unwrap(),
			n => writeln!(self.out, "find matches {n} modules (all: true)")
				.unwrap(),
		}
		for &id in ids.iter().take(self.max_modules) {
			self.apply(patch, id);
		}
		if ids.len() > self.max_modules {
			writeln!(
				self.out,
				"[{} more matching modules not tested]",
				ids.len() - self.max_modules
			)
			.unwrap();
		}
	}

	fn apply(&mut self, patch: &Patch, id: ModuleId) {
		let Ok(raw) = self.bundle.raw(id) else {
			return;
		};
		let original = format!("0,{raw}");
		let file_name = format!("{id}.js");
		self.alloc.reset();
		let applied = apply_patch(
			&mut self.alloc,
			patch,
			original.clone(),
			&ApplyOptions {
				plugin_name: Some(self.name),
				syntax_errors: SyntaxErrorReport::Formatted {
					file_name: Some(&file_name),
				},
				..ApplyOptions::default()
			},
		);
		let errors = applied
			.events
			.into_iter()
			.map(|e| ReporterError::from_event(e, patch.plugin_id(), id))
			.collect::<Vec<_>>();
		let failed = errors.iter().any(is_error);
		writeln!(
			self.out,
			"\n### module {id}: {}",
			if failed { "FAIL" } else { "OK" }
		)
		.unwrap();
		for e in errors {
			let prefix = if e.is_no_warn() { "(noWarn) " } else { "" };
			writeln!(self.out, "{prefix}{}", render_error(e, self.plugins))
				.unwrap();
		}
		let old =
			pretty_printer::format_to_str(&original, 4).unwrap_or(original);
		let new = pretty_printer::format_to_str(&applied.src, 4)
			.unwrap_or(applied.src);
		let diff = unified_diff(&old, &new, self.context);
		if diff.is_empty() {
			self.out
				.push_str("(no change to the module)\n");
		} else {
			writeln!(self.out, "```diff\n{diff}```").unwrap();
		}
	}
}

pub async fn test_patch(
	b: Arc<LoadedBundle>,
	input: PluginInput,
	patch_index: Option<usize>,
	max_modules: usize,
	context: usize,
) -> Result<String> {
	let (path, source) = input.read().await?;
	let path_str = path.to_string_lossy().into_owned();
	let name = plugin_name(&source, &path_str);
	let plugins = collect_plugins_from_paths(vec![(path, source)])
		.await
		.map_err(|e| anyhow!("Failed to parse the plugin:\n{e:?}"))?;
	let plugins = Arc::new(plugins);
	task::spawn_blocking(move || {
		let patches = &plugins[0].patches;
		if patches.is_empty() {
			bail!("The plugin has no patches");
		}
		if let Some(i) = patch_index
			&& i >= patches.len()
		{
			bail!(
				"patch_index {i} is out of range, the plugin has {} patches",
				patches.len()
			);
		}
		let mut run = PatchRun {
			bundle: &b,
			plugins: &plugins,
			name: &name,
			max_modules,
			context,
			alloc: Allocator::default(),
			out: format!(
				"testing {} patch(es) of {name} against build {}\n",
				patch_index.map_or(patches.len(), |_| 1),
				b.id
			),
		};
		if b.preformatted {
			run.out.push_str(
				"Note: this build's modules are pretty printed, so patches \
				 written against the minified client may not match.\n",
			);
		}
		for (i, patch) in patches.iter().enumerate() {
			if patch_index.is_none_or(|want| want == i) {
				run.run(i, patch);
			}
		}
		Ok(truncate(
			run.out,
			"Test one patch at a time with patch_index, or lower context",
		))
	})
	.await
	.context("Testing patches panicked")?
}

/// Whether `map`, or any object it exports, has every one of `props`
fn has_props(map: &RangeExportMap, props: &[&str]) -> bool {
	let direct = |m: &RangeExportMap| {
		props
			.iter()
			.all(|p| m.exports.contains_key(*p))
	};
	direct(map)
		|| map
			.exports
			.values()
			.chain(map.cjs_default.as_deref())
			.any(|v| matches!(v, ExportValue::Map(m) if direct(m)))
}

fn string_args(args: &[FindArg]) -> Option<Vec<&str>> {
	args.iter()
		.map(|a| match a {
			FindArg::String(s) => Some(s.as_str()),
			_ => None,
		})
		.collect()
}

async fn eval_find(
	b: &Arc<LoadedBundle>,
	find: &FindUse,
	progress: &Arc<dyn ScrapeProgress>,
) -> Result<Vec<ModuleId>> {
	let args = &find.data.args;
	match find.data.kind.find_type {
		FindType::ModuleId => {
			let [FindArg::String(id)] = args.as_slice() else {
				bail!("expected one module id");
			};
			let id = crate::store::parse_module_id(id)?;
			Ok(if b.modules.contains_key(&id) {
				vec![id]
			} else {
				Vec::new()
			})
		}
		FindType::ByCode | FindType::ComponentByCode => {
			let matchers = args
				.iter()
				.map(|a| match a {
					FindArg::String(s) => Matcher::parse_find(s),
					FindArg::Regex { pattern, flags } => {
						Matcher::regex(pattern, flags)
					}
					FindArg::Function(_) => {
						bail!("filter functions can't be checked offline")
					}
				})
				.collect::<Result<Vec<_>>>()?;
			let mut ids = b
				.modules
				.par_iter()
				.filter(|(_, src)| matchers.iter().all(|m| m.is_match(src)))
				.map(|(&id, _)| id)
				.collect::<Vec<_>>();
			ids.sort_unstable();
			Ok(ids)
		}
		FindType::ByProps => {
			let props = string_args(args).context("expected string props")?;
			let finders = props
				.iter()
				.map(|p| Matcher::parse_find(p))
				.collect::<Result<Vec<_>>>()?;
			let parsed = b.parsed(progress.clone()).await?;
			let mut ids = b
				.modules
				.iter()
				.filter(|(_, src)| finders.iter().all(|f| f.is_match(src)))
				.filter(|&(&id, _)| {
					parsed.get_parser(id).is_some_and(|p| {
						has_props(p.parser().get_export_map(), &props)
					})
				})
				.map(|(&id, _)| id)
				.collect::<Vec<_>>();
			ids.sort_unstable();
			Ok(ids)
		}
		FindType::Store => {
			let [FindArg::String(name)] = args.as_slice() else {
				bail!("expected one store name");
			};
			let parsed = b.parsed(progress.clone()).await?;
			let finder = Matcher::parse_find(name)?;
			let mut ids = parsed
				.parsers()
				.filter(|(id, _)| {
					b.raw(*id)
						.is_ok_and(|src| finder.is_match(src))
				})
				.filter(|(_, p)| store_named(p.parser().get_export_map(), name))
				.map(|(id, _)| id)
				.collect::<Vec<_>>();
			ids.sort_unstable();
			Ok(ids)
		}
		FindType::CssClasses => {
			let classes = string_args(args).context("expected class names")?;
			let all = super::misc::all_css_classes(b, progress.clone()).await?;
			let mut by_module = BTreeMap::<ModuleId, HashSet<&str>>::new();
			for c in all.iter() {
				by_module
					.entry(c.module)
					.or_default()
					.insert(&c.exported_as);
			}
			Ok(by_module
				.into_iter()
				.filter(|(_, names)| {
					classes
						.iter()
						.all(|c| names.contains(c))
				})
				.map(|(id, _)| id)
				.collect())
		}
		FindType::Component => {
			bail!("filter functions can't be checked offline")
		}
	}
}

fn store_named(map: &RangeExportMap, name: &str) -> bool {
	(map.extra_data.is_store() && map.hover.as_deref() == Some(name))
		|| map
			.exports
			.values()
			.chain(map.cjs_default.as_deref())
			.any(|v| matches!(v, ExportValue::Map(m) if store_named(m, name)))
}

fn describe_find(src: &str, find: &FindUse) -> String {
	let text = src
		.get(find.range.start as usize..find.range.end as usize)
		.unwrap_or("?");
	let text = text
		.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ");
	if text.len() > 160 {
		format!("{}…", &text[..text.floor_char_boundary(160)])
	} else {
		text
	}
}

pub async fn test_find(
	b: Arc<LoadedBundle>,
	input: PluginInput,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	let (path, source) = input.read().await?;
	let path_str = path.to_string_lossy().into_owned();
	let finds = {
		let alloc = Allocator::default();
		let parser =
			VencordAstParser::try_new(&alloc, &source, Some(&path_str))
				.map_err(|e| anyhow!("Failed to parse the plugin:\n{e:?}"))?;
		match parser.get_finds() {
			Ok(finds) => finds
				.iter()
				.map(clone_find)
				.collect::<Vec<_>>(),
			Err(e) => bail!("Failed to collect finds: {e}"),
		}
	};
	if finds.is_empty() {
		return Ok(String::from("The plugin has no webpack finds"));
	}
	let lines = LineIndex::new(&source);
	let mut out = format!(
		"checking {} find(s) against build {}. findByCode is approximated by \
		 whole module text, so a match may still fail if the code is split \
		 across exports.\n",
		finds.len(),
		b.id
	);
	for find in &finds {
		write!(
			out,
			"\nline {}: {}\n  ",
			lines.line(find.range.start as usize),
			describe_find(&source, find)
		)
		.unwrap();
		match eval_find(&b, find, &progress).await {
			Ok(ids) => match ids.len() {
				0 => out.push_str("BROKEN: matches no module\n"),
				1 => writeln!(out, "OK: module {}", ids[0]).unwrap(),
				n => writeln!(
					out,
					"AMBIGUOUS: {n} modules match ({}). Vencord uses the \
					 first one it finds, which may be the wrong one.",
					id_list(&ids, 10)
				)
				.unwrap(),
			},
			Err(e) => writeln!(out, "SKIPPED: {e:#}").unwrap(),
		}
	}
	Ok(truncate(out, "The plugin has too many finds to show"))
}

fn clone_find(find: &FindUse) -> FindUse {
	use vencord_ast_parser::{AnyFindType, FindData};
	FindUse {
		range: find.range,
		data: FindData {
			kind: AnyFindType {
				find_type: find.data.kind.find_type,
				lazy: find.data.kind.lazy,
			},
			args: find
				.data
				.args
				.iter()
				.map(|a| match a {
					FindArg::String(s) => FindArg::String(s.clone()),
					FindArg::Regex { pattern, flags } => FindArg::Regex {
						pattern: pattern.clone(),
						flags: flags.clone(),
					},
					FindArg::Function(f) => FindArg::Function(f.clone()),
				})
				.collect(),
		},
	}
}

pub async fn check_plugins(
	b: Arc<LoadedBundle>,
	vencord_dir: String,
	plugin_dirs: Vec<String>,
	include_warnings: bool,
	limit: usize,
) -> Result<String> {
	let opts = VencordOpts {
		vencord_dir: PathBuf::from(&vencord_dir),
		plugin_dirs: plugin_dirs
			.into_iter()
			.map(PathBuf::from)
			.collect(),
	};
	let plugins = Arc::new(
		collect_patches(opts, Stage::hidden())
			.await
			.context("Failed to collect plugins")?,
	);
	let num_patches = plugins
		.iter()
		.map(|p| p.patches.len())
		.sum::<usize>();
	let channel = b
		.channels
		.first()
		.copied()
		.unwrap_or(Channel::Stable);
	let mut rx =
		report_broken_patches(channel, b.modules.clone(), plugins.clone());
	let mut errors = Vec::new();
	let mut warnings = 0;
	while let Some(msg) = rx.recv().await {
		match msg {
			Msg::RequestProgressBar(tx) => {
				_ = tx.send(MultiProgressWrapper::null_bar());
			}
			Msg::Done(_) => break,
			Msg::Error(e) => {
				if e.is_no_warn() {
					continue;
				}
				if e.severity() == Some(Severity::Warning) {
					warnings += 1;
					if !include_warnings {
						continue;
					}
				}
				errors.push(e);
			}
		}
	}
	let mut out = format!(
		"checked {num_patches} patch(es) in {} plugin(s) against build {}: \
		 {} problem(s)",
		plugins.len(),
		b.id,
		errors.len()
	);
	if !include_warnings && warnings > 0 {
		write!(out, ", {warnings} warning(s) hidden").unwrap();
	}
	out.push('\n');
	if b.preformatted {
		out.push_str(
			"Note: this build's modules are pretty printed, so patches \
			 written against the minified client may not match.\n",
		);
	}
	let total = errors.len();
	for e in errors.into_iter().take(limit) {
		writeln!(out, "\n{}", render_error(e, &plugins)).unwrap();
	}
	if total > limit {
		write!(out, "\n[{} more not shown. Raise limit]", total - limit)
			.unwrap();
	}
	Ok(truncate(
		out,
		"Raise limit carefully, or test single plugins with test_patch",
	))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::store::test_util::bundle;

	const MODULES: &[(u32, &str)] = &[
		(1, "function(e,t,n){n.d(t,{Z:()=>r});let r=()=>\"isStaff\"}"),
		(2, "function(e,t,n){let r=\"other\"}"),
	];

	const PLUGIN: &str = r#"
import definePlugin from "@utils/types";

export default definePlugin({
    name: "TestPlugin",
    description: "test",
    authors: [],
    patches: [
        {
            find: '"isStaff"',
            replacement: {
                match: /"isStaff"/,
                replace: '"isStaff"+$self.x'
            }
        },
        {
            find: "nonexistent_marker",
            replacement: {
                match: /a/,
                replace: "b"
            }
        },
        {
            find: '"isStaff"',
            replacement: {
                match: /does_not_match/,
                replace: "b"
            }
        }
    ]
});
"#;

	fn input() -> PluginInput {
		PluginInput {
			path: None,
			source: Some(PLUGIN.to_owned()),
		}
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn reports_each_patch() {
		let b = bundle("abc", MODULES);
		let out = test_patch(b, input(), None, 3, 2)
			.await
			.unwrap();
		let sections = out
			.split("\n## patch ")
			.collect::<Vec<_>>();
		assert_eq!(sections.len(), 4, "{out}");
		assert!(sections[1].contains("### module 1: OK"), "{out}");
		assert!(
			sections[1].contains(r#"Vencord.Plugins.plugins["TestPlugin"]"#),
			"{out}"
		);
		assert!(sections[2].contains("matches no module"), "{out}");
		assert!(sections[3].contains("### module 1: FAIL"), "{out}");
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn tests_single_patch() {
		let b = bundle("abc", MODULES);
		let out = test_patch(b.clone(), input(), Some(1), 3, 2)
			.await
			.unwrap();
		assert_eq!(out.matches("\n## patch ").count(), 1, "{out}");
		assert!(
			test_patch(b, input(), Some(9), 3, 2)
				.await
				.is_err()
		);
	}
}
