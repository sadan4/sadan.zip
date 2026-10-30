//! Exports, references and flux stores, all of which need the build parsed

use std::{collections::BTreeMap, fmt::Write as _, sync::Arc};

use anyhow::{Context as _, Result, anyhow, bail};
use discord_scraper::{ScrapeProgress, experiments::ParsedBundle};
use explorer_types::ModuleId;
use oxc::span::Span;
use smol_str::SmolStr;
use webpack_ast_parser::{
	ThreadSafeParser,
	export_map::{
		ExportMapKey,
		ExportRange,
		ExportValue,
		ExtraData,
		RangeExportMap,
		StoreData,
	},
};

use crate::{
	store::{FormattedModule, LoadedBundle},
	text::{first_line, truncate},
};

fn parser_for(
	parsed: &ParsedBundle,
	id: ModuleId,
) -> Result<Arc<ThreadSafeParser>> {
	parsed
		.get_parser(id)
		.with_context(|| format!("Module {id} not found"))
}

/// The display text of `span` on its formatted line
fn span_text(fmt: &FormattedModule, span: Span, max: usize) -> String {
	let start = fmt.map_pos(span.start);
	let end = fmt.map_pos(span.end.saturating_sub(1)) + 1;
	first_line(&fmt.code, start..end.max(start), max).to_owned()
}

fn key_name(key: &ExportMapKey) -> &str {
	match key {
		ExportMapKey::Named(name) => name,
		ExportMapKey::Default => "default",
	}
}

fn sorted_exports(map: &RangeExportMap) -> Vec<(&SmolStr, &ExportValue<Span>)> {
	let mut exports = map.exports.iter().collect::<Vec<_>>();
	exports.sort_unstable_by(|a, b| a.0.cmp(b.0));
	exports
}

struct Renderer<'a> {
	fmt: &'a FormattedModule,
	out: String,
}

impl Renderer<'_> {
	fn map(&mut self, map: &RangeExportMap, depth: usize) {
		let indent = "  ".repeat(depth);
		if let Some(hover) = &map.hover {
			writeln!(self.out, "{indent}(display name: {hover})").unwrap();
		}
		if let ExtraData::Store(store) = &map.extra_data {
			self.store(store, depth);
		}
		if let Some(default) = &map.cjs_default {
			self.value("default", default, depth);
		}
		for (name, value) in sorted_exports(map) {
			self.value(name, value, depth);
		}
	}

	fn store(&mut self, store: &StoreData<Span>, depth: usize) {
		let indent = "  ".repeat(depth);
		let mut events = store
			.flux_events
			.iter()
			.collect::<Vec<_>>();
		events.sort_unstable_by(|a, b| a.0.cmp(b.0));
		writeln!(
			self.out,
			"{indent}flux store, defined at line {}, handles {} event(s):",
			self.fmt.line_of(store.store.start),
			events.len()
		)
		.unwrap();
		for (event, span) in events {
			writeln!(
				self.out,
				"{indent}  {event} (line {})",
				self.fmt.line_of(span.start)
			)
			.unwrap();
		}
	}

	fn value(&mut self, name: &str, value: &ExportValue<Span>, depth: usize) {
		let indent = "  ".repeat(depth);
		match value {
			ExportValue::Range(ExportRange(spans, hover)) => {
				write!(self.out, "{indent}{name}").unwrap();
				// the first span is where it's exported, the last is where
				// the value is defined
				match (spans.first(), spans.last()) {
					(Some(&first), Some(&last)) if first != last => write!(
						self.out,
						" (line {}, defined at line {}): {}",
						self.fmt.line_of(first.start),
						self.fmt.line_of(last.start),
						span_text(self.fmt, last, 100)
					)
					.unwrap(),
					(Some(&span), _) => write!(
						self.out,
						" (line {}): {}",
						self.fmt.line_of(span.start),
						span_text(self.fmt, span, 100)
					)
					.unwrap(),
					_ => {}
				}
				if let Some(hover) = hover {
					write!(self.out, "  [{hover}]").unwrap();
				}
				self.out.push('\n');
			}
			ExportValue::Map(map) => {
				writeln!(self.out, "{indent}{name}:").unwrap();
				self.map(map, depth + 1);
			}
		}
	}
}

pub async fn module_exports(
	b: Arc<LoadedBundle>,
	id: ModuleId,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	b.raw(id)?;
	let parsed = b.parsed(progress).await?;
	let parser = parser_for(&parsed, id)?;
	let map = parser.parser().get_export_map();
	let fmt = b.formatted(id)?;
	if map.is_empty() && map.extra_data.is_none() {
		return Ok(format!(
			"Module {id} has no exports that could be found. Line numbers in \
			 other tools match get_module with pretty=true."
		));
	}
	let mut r = Renderer {
		fmt: &fmt,
		out: format!(
			"exports of module {id} (line numbers match get_module with \
			 pretty=true):\n"
		),
	};
	r.map(map, 0);
	Ok(truncate(r.out, "This module has too many exports to show"))
}

/// Walks `path` through `map`, `default` meaning the default export
fn resolve_export<'a>(
	map: &'a RangeExportMap,
	path: &[String],
) -> Result<&'a ExportValue<Span>> {
	let [first, rest @ ..] = path else {
		bail!("export_path must name at least one export");
	};
	let key = if first == "default" {
		ExportMapKey::Default
	} else {
		ExportMapKey::Named(first.into())
	};
	let value = map.get(&key).with_context(|| {
		let mut names = map
			.exports
			.keys()
			.map(SmolStr::as_str)
			.collect::<Vec<_>>();
		names.sort_unstable();
		if map.cjs_default.is_some() {
			names.insert(0, "default");
		}
		format!(
			"No export named {:?}. Exports here: {}",
			key_name(&key),
			names.join(", ")
		)
	})?;
	match (value, rest) {
		(value, []) => Ok(value),
		(ExportValue::Map(map), rest) => resolve_export(map, rest),
		(ExportValue::Range(_), _) => {
			bail!(
				"{:?} is not an object, so it has no members",
				key_name(&key)
			)
		}
	}
}

fn first_span(value: &ExportValue<Span>) -> Option<Span> {
	match value {
		ExportValue::Range(range) => range.0.first().copied(),
		ExportValue::Map(map) => map
			.cjs_default
			.as_deref()
			.and_then(first_span)
			.or_else(|| {
				sorted_exports(map)
					.into_iter()
					.find_map(|(_, v)| first_span(v))
			}),
	}
}

pub async fn find_export_uses(
	b: Arc<LoadedBundle>,
	id: ModuleId,
	export_path: &[String],
	limit: usize,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	b.raw(id)?;
	let parsed = b.parsed(progress).await?;
	let parser = parser_for(&parsed, id)?;
	let value = resolve_export(parser.parser().get_export_map(), export_path)?;
	let span = first_span(value)
		.context("Could not find where this export is defined")?;
	let refs = parser
		.parser()
		.generate_references(span.start)
		.await
		.map_err(|e| anyhow!("{e}"))?;
	let path = export_path.join(".");
	if refs.is_empty() {
		return Ok(format!("No module uses {path} of module {id}"));
	}
	let mut by_module = BTreeMap::<ModuleId, Vec<Span>>::new();
	for r in refs {
		by_module
			.entry(r.module_id)
			.or_default()
			.push(r.range);
	}
	let total = by_module
		.values()
		.map(Vec::len)
		.sum::<usize>();
	let mut out = format!(
		"{total} use(s) of {path} from module {id} across {} module(s). \
		 Line numbers match get_module with pretty=true.\n",
		by_module.len()
	);
	let mut shown = 0;
	'outer: for (module, mut spans) in by_module {
		spans.sort_unstable_by_key(|s| s.start);
		let fmt = b.formatted(module)?;
		writeln!(out, "\nmodule {module}:").unwrap();
		for span in spans {
			if shown == limit {
				break 'outer;
			}
			shown += 1;
			writeln!(
				out,
				"  line {}: {}",
				fmt.line_of(span.start),
				span_text(&fmt, span, 120)
			)
			.unwrap();
		}
	}
	if shown < total {
		write!(out, "\n[{} more not shown. Raise limit]", total - shown)
			.unwrap();
	}
	Ok(truncate(out, "Lower limit"))
}

struct FoundStore {
	module: ModuleId,
	path: String,
	name: Option<SmolStr>,
	events: Vec<SmolStr>,
}

fn collect_stores(
	map: &RangeExportMap,
	module: ModuleId,
	path: &str,
	out: &mut Vec<FoundStore>,
) {
	if let ExtraData::Store(store) = &map.extra_data {
		let mut events = store
			.flux_events
			.keys()
			.cloned()
			.collect::<Vec<_>>();
		events.sort_unstable();
		out.push(FoundStore {
			module,
			path: if path.is_empty() {
				String::from("(module)")
			} else {
				path.to_owned()
			},
			name: map.hover.clone(),
			events,
		});
	}
	let children = map
		.cjs_default
		.as_deref()
		.map(|v| ("default", v))
		.into_iter()
		.chain(
			sorted_exports(map)
				.into_iter()
				.map(|(k, v)| (k.as_str(), v)),
		);
	for (name, value) in children {
		if let ExportValue::Map(child) = value {
			let path = if path.is_empty() {
				name.to_owned()
			} else {
				format!("{path}.{name}")
			};
			collect_stores(child, module, &path, out);
		}
	}
}

pub async fn find_store(
	b: Arc<LoadedBundle>,
	name: &str,
	limit: usize,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	let parsed = b.parsed(progress).await?;
	let mut stores = Vec::new();
	for (id, parser) in parsed.parsers() {
		collect_stores(parser.parser().get_export_map(), id, "", &mut stores);
	}
	let needle = name.trim().to_ascii_lowercase();
	let mut found = stores
		.into_iter()
		.filter(|s| {
			needle.is_empty()
				|| s.name
					.as_ref()
					.is_some_and(|n| n.to_ascii_lowercase().contains(&needle))
		})
		.collect::<Vec<_>>();
	// exact names first, then by name
	found.sort_by(|a, b| {
		let exact = |s: &FoundStore| {
			s.name
				.as_ref()
				.is_none_or(|n| !n.eq_ignore_ascii_case(name.trim()))
		};
		(exact(a), &a.name, a.module).cmp(&(exact(b), &b.name, b.module))
	});
	if found.is_empty() {
		return Ok(format!("No flux store named like {name:?}"));
	}
	let mut out = format!("{} store(s) found\n", found.len());
	for s in found.iter().take(limit) {
		writeln!(
			out,
			"\n{} - module {}, export {}",
			s.name.as_deref().unwrap_or("(unnamed)"),
			s.module,
			s.path
		)
		.unwrap();
		if !s.events.is_empty() {
			writeln!(out, "  handles: {}", s.events.join(", ")).unwrap();
		}
	}
	if found.len() > limit {
		write!(out, "\n[{} more not shown]", found.len() - limit).unwrap();
	}
	Ok(truncate(out, "Use a more specific name"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::store::test_util::bundle;
	use discord_scraper::NoProgress;

	const MODULES: &[(u32, &str)] = &[
		(
			1,
			"function(e,t,n){n.d(t,{Z:()=>r,Y:()=>o});let r=function(){return 1},o={a:1}}",
		),
		(2, "function(e,t,n){let a=n(1);a.Z();a.Z()}"),
	];

	#[tokio::test]
	async fn lists_exports() {
		let b = bundle("abc", MODULES);
		let out = module_exports(b, ModuleId(1), Arc::new(NoProgress))
			.await
			.unwrap();
		assert!(out.contains("Z (line"), "{out}");
		assert!(out.contains('Y'), "{out}");
	}

	#[tokio::test]
	async fn finds_uses_of_export() {
		let b = bundle("abc", MODULES);
		let out = find_export_uses(
			b,
			ModuleId(1),
			&[String::from("Z")],
			50,
			Arc::new(NoProgress),
		)
		.await
		.unwrap();
		assert!(out.contains("module 2:"), "{out}");
	}

	#[tokio::test]
	async fn missing_export_lists_names() {
		let b = bundle("abc", MODULES);
		let err = find_export_uses(
			b,
			ModuleId(1),
			&[String::from("nope")],
			50,
			Arc::new(NoProgress),
		)
		.await
		.unwrap_err();
		assert!(
			err.to_string()
				.contains("Exports here: Y, Z"),
			"{err}"
		);
	}
}
