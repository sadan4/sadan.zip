//! Reading, searching and diffing modules

use std::{fmt::Write as _, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use explorer_types::ModuleId;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use term_progress::MultiProgressWrapper;
use tokio::task;

use super::{Matcher, id_list};
use crate::{
	store::LoadedBundle,
	text::{LineIndex, number_lines, snippet, truncate, unified_diff},
};

/// Lines returned by [`get_module`] when no range is given
pub const DEFAULT_LINES: usize = 300;

pub fn get_module(
	b: &LoadedBundle,
	id: ModuleId,
	pretty: bool,
	start_line: Option<usize>,
	end_line: Option<usize>,
) -> Result<String> {
	let fmt;
	let raw;
	let (code, lines): (&str, &LineIndex) = if pretty {
		fmt = b.formatted(id)?;
		(&fmt.code, &fmt.lines)
	} else {
		let src = b.with_header(id)?;
		raw = (LineIndex::new(&src), src);
		(&raw.1, &raw.0)
	};
	let total = lines.len();
	let start = start_line.unwrap_or(1).clamp(1, total);
	let end = end_line
		.unwrap_or(start + DEFAULT_LINES - 1)
		.clamp(start, total);
	let range = lines.range(code.len(), start, end);
	let mut out = format!(
		"// module {id}, build {}, lines {start}-{end} of {total}",
		b.id
	);
	if !pretty {
		out.push_str(" (unformatted)");
	}
	out.push('\n');
	out.push_str(&number_lines(&code[range], start));
	if end < total {
		write!(
			out,
			"\n[{} more lines. Call again with start_line={}]",
			total - end,
			end + 1
		)
		.unwrap();
	}
	Ok(truncate(
		out,
		"Request a smaller line range with start_line/end_line",
	))
}

pub fn search_modules(
	b: &LoadedBundle,
	query: &str,
	regex: bool,
	limit: usize,
	context: usize,
) -> Result<String> {
	let matcher = if regex {
		Matcher::regex(query, "")?
	} else {
		Matcher::parse_find(query)?
	};
	let mut hits = b
		.modules
		.par_iter()
		.filter_map(|(&id, src)| {
			let first = matcher.find(src)?;
			Some((id, first, src.as_str()))
		})
		.collect::<Vec<_>>();
	hits.sort_unstable_by_key(|&(id, ..)| id);
	let total = hits.len();
	let mut out = format!("{total} module(s) match in build {}\n", b.id);
	for (id, range, src) in hits.iter().take(limit) {
		writeln!(out, "\n{id}: {}", snippet(src, range.clone(), context))
			.unwrap();
	}
	if total > limit {
		write!(
			out,
			"\n[{} more not shown. Narrow the query or raise limit]",
			total - limit
		)
		.unwrap();
	}
	Ok(truncate(out, "Narrow the query or lower limit/context"))
}

pub fn find_module(b: &LoadedBundle, find: &str) -> Result<String> {
	let matcher = Matcher::parse_find(find)?;
	let ids = matcher.matching_modules(b);
	let mut out = match ids.len() {
		0 => String::from(
			"No module matches this find. It is broken on this build.\n",
		),
		1 => String::from("Exactly one module matches. This find is unique.\n"),
		n => format!(
			"{n} modules match. This find is ambiguous: a patch using it \
			 needs `all: true` or a more specific find.\n"
		),
	};
	writeln!(out, "matches: {}", id_list(&ids, 50)).unwrap();
	for &id in ids.iter().take(10) {
		let src = b.raw(id)?;
		if let Some(range) = matcher.find(src) {
			writeln!(out, "\n{id}: {}", snippet(src, range, 80)).unwrap();
		}
	}
	if b.preformatted {
		out.push_str(
			"\nNote: this build's modules are pretty printed, so finds \
			 written against the minified client may not match.\n",
		);
	}
	Ok(truncate(out, "The find matches too many modules"))
}

pub async fn generate_finds(
	b: Arc<LoadedBundle>,
	id: ModuleId,
	limit: usize,
) -> Result<String> {
	let src = b.with_header(id)?;
	let finds = {
		let b = b.clone();
		task::spawn_blocking(move || {
			reporter::util::generate_unique_finds(
				id,
				&b.modules,
				&MultiProgressWrapper::null_bar(),
			)
			.map_err(|e| anyhow!("{e:?}"))
		})
		.await
		.context("Generating finds panicked")??
	};
	if finds.is_empty() {
		return Ok(format!(
			"No unique find could be generated for module {id}"
		));
	}
	let mut out = format!(
		"{} unique find(s) for module {id}, best first. Each matches only \
		 this module in build {}.\n",
		finds.len(),
		b.id
	);
	for find in finds.iter().take(limit) {
		let mut text = find.get_find(&src).to_owned();
		for key in &find.intl_keys {
			if let Some(name) = &key.unhashed {
				let vencord = format!("#{{intl::{name}}}");
				text = text
					.replace(&format!("[\"{}\"]", key.hashed), &vencord)
					.replace(&format!(".{}", key.hashed), &vencord);
			}
		}
		let literal = serde_json::to_string(&text)?;
		writeln!(out, "\nscore {}: {literal}", find.score).unwrap();
	}
	Ok(truncate(out, "Lower limit"))
}

#[derive(
	Debug, Clone, Copy, Default, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum DepDirection {
	/// modules this module requires
	Out,
	/// modules that require this module
	In,
	#[default]
	Both,
}

pub async fn module_deps(
	b: Arc<LoadedBundle>,
	id: ModuleId,
	direction: DepDirection,
) -> Result<String> {
	b.raw(id)?;
	let mut out = format!("dependencies of module {id} in build {}\n", b.id);
	if matches!(direction, DepDirection::Out | DepDirection::Both) {
		let parser = b.standalone_parser(id)?;
		let (sync, lazy) = parser
			.parser()
			.get_modules_that_this_module_requires()
			.map_or_else(Default::default, |deps| {
				let mut sync = deps
					.sync
					.iter()
					.map(|d| d.id)
					.collect::<Vec<_>>();
				let mut lazy = deps
					.lazy
					.iter()
					.map(|d| d.id)
					.collect::<Vec<_>>();
				sync.sort_unstable();
				sync.dedup();
				lazy.sort_unstable();
				lazy.dedup();
				(sync, lazy)
			});
		writeln!(out, "requires: {}", id_list(&sync, 200)).unwrap();
		writeln!(out, "requires lazily: {}", id_list(&lazy, 200)).unwrap();
	}
	if matches!(direction, DepDirection::In | DepDirection::Both) {
		let mut deps = b.incoming_deps(id).await?;
		deps.sync.sort_unstable();
		deps.lazy.sort_unstable();
		writeln!(out, "required by: {}", id_list(&deps.sync, 200)).unwrap();
		writeln!(out, "required lazily by: {}", id_list(&deps.lazy, 200))
			.unwrap();
	}
	Ok(truncate(out, "Request one direction at a time"))
}

pub fn diff_module(
	old: &LoadedBundle,
	new: &LoadedBundle,
	id: ModuleId,
	context: usize,
) -> Result<String> {
	let header = format!("module {id}: build {} -> build {}", old.id, new.id);
	match (old.raw(id).is_ok(), new.raw(id).is_ok()) {
		(true, true) => {}
		(false, false) => {
			return Ok(format!("{header}\nModule {id} is in neither build"));
		}
		(true, false) => {
			return Ok(format!("{header}\nModule {id} was removed"));
		}
		(false, true) => {
			return Ok(format!("{header}\nModule {id} was added"));
		}
	}
	if old.raw(id)? == new.raw(id)? {
		return Ok(format!("{header}\nidentical"));
	}
	let old_fmt = old.formatted(id)?;
	let new_fmt = new.formatted(id)?;
	let diff = unified_diff(&old_fmt.code, &new_fmt.code, context);
	if diff.is_empty() {
		return Ok(format!("{header}\nonly whitespace differs"));
	}
	Ok(truncate(format!("{header}\n{diff}"), "Lower context"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::store::test_util::bundle;

	const MODULES: &[(u32, &str)] = &[
		(
			1,
			"function(e,t,n){n.d(t,{Z:()=>r});let r=\"unique_marker\"}",
		),
		(2, "function(e,t,n){let a=n(1),r=\"shared\";a.Z}"),
		(3, "function(e,t,n){let r=\"shared\"}"),
	];

	#[test]
	fn find_module_reports_uniqueness() {
		let b = bundle("abc", MODULES);
		let out = find_module(&b, "unique_marker").unwrap();
		assert!(out.contains("Exactly one module"), "{out}");
		let out = find_module(&b, "shared").unwrap();
		assert!(out.contains("2 modules match"), "{out}");
		let out = find_module(&b, "nowhere").unwrap();
		assert!(out.contains("No module matches"), "{out}");
	}

	#[test]
	fn get_module_pages_lines() {
		let b = bundle("abc", MODULES);
		let out = get_module(&b, ModuleId(1), true, Some(1), Some(2)).unwrap();
		assert!(out.contains("lines 1-2 of"), "{out}");
		assert!(out.contains("Call again with start_line=3"), "{out}");
		assert!(get_module(&b, ModuleId(9), true, None, None).is_err());
	}

	#[test]
	fn search_limits_results() {
		let b = bundle("abc", MODULES);
		let out = search_modules(&b, "shared", false, 1, 10).unwrap();
		assert!(out.starts_with("2 module(s)"), "{out}");
		assert!(out.contains("1 more not shown"), "{out}");
	}

	#[tokio::test]
	async fn deps_from_parser() {
		let b = bundle("abc", MODULES);
		let out = module_deps(b, ModuleId(2), DepDirection::Out)
			.await
			.unwrap();
		assert!(out.contains("requires: 1"), "{out}");
	}

	#[test]
	fn diff_between_builds() {
		let a = bundle("a", MODULES);
		let b = bundle("b", &[(1, "function(e,t,n){let r=\"changed\"}")]);
		let out = diff_module(&a, &b, ModuleId(1), 2).unwrap();
		assert!(out.contains('+'), "{out}");
		let out = diff_module(&a, &b, ModuleId(2), 2).unwrap();
		assert!(out.contains("removed"), "{out}");
	}
}
