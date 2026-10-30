//! Searching and reading Discord's css

use std::{collections::HashSet, fmt::Write as _, sync::Arc};

use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Matcher;
use crate::{
	css::{CssBlock, CssChunk, CssIndex},
	store::LoadedBundle,
	text::{LineIndex, number_lines, truncate},
};

/// Lines of a block shown before it's cut off
const BLOCK_LINES: usize = 15;
/// Lines returned by [`get_css`] when no range is given
const DEFAULT_LINES: usize = 300;

#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CssTarget {
	/// anywhere in the css
	#[default]
	Any,
	/// only selectors and at-rule preludes, eg to find every rule for a class
	Selector,
	/// only declarations, eg `color:` or a variable name
	Declaration,
}

fn matcher(query: &str, regex: bool) -> Result<Matcher> {
	if regex {
		Matcher::regex(query, "")
	} else {
		Matcher::parse_find(query)
	}
}

/// Writes `block` with its location, cut down to [`BLOCK_LINES`]
fn render_block(out: &mut String, chunk: &CssChunk, block: &CssBlock) {
	let line = chunk.block_line(block);
	write!(out, "\n{}:{line}", chunk.name).unwrap();
	if !block.context.is_empty() {
		write!(out, " (inside {})", block.context.join(" > ")).unwrap();
	}
	out.push('\n');
	let text = &chunk.code[block.range.clone()];
	let indent = block.context.len() * 2;
	let total = text.lines().count();
	for l in text.lines().take(BLOCK_LINES) {
		out.push_str(l.get(indent..).unwrap_or(l));
		out.push('\n');
	}
	if total > BLOCK_LINES {
		writeln!(
			out,
			"… {} more lines (get_css chunk={} start_line={})",
			total - BLOCK_LINES,
			chunk.name,
			line + BLOCK_LINES
		)
		.unwrap();
	}
}

/// Where a match landed: a block, or a top level statement like `@import`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Hit {
	Block(usize, usize),
	Line(usize, usize),
}

fn collect_hits(
	css: &CssIndex,
	matcher: &Matcher,
	target: CssTarget,
) -> Vec<Hit> {
	let mut hits = Vec::new();
	for (ci, chunk) in css.chunks.iter().enumerate() {
		match target {
			CssTarget::Any => {
				let mut seen = HashSet::new();
				for m in matcher.find_iter(&chunk.code) {
					let hit = chunk.block_at(m.start).map_or_else(
						|| Hit::Line(ci, chunk.lines.line(m.start)),
						|bi| Hit::Block(ci, bi),
					);
					if seen.insert(hit) {
						hits.push(hit);
					}
				}
			}
			CssTarget::Selector => {
				hits.extend(
					chunk
						.blocks
						.iter()
						.enumerate()
						.filter(|(_, b)| matcher.is_match(&b.prelude))
						.map(|(bi, _)| Hit::Block(ci, bi)),
				);
			}
			CssTarget::Declaration => {
				hits.extend(
					chunk
						.blocks
						.iter()
						.enumerate()
						.filter(|(_, b)| {
							!b.has_children
								&& chunk.code[b.body.clone()]
									.lines()
									.any(|l| matcher.is_match(l))
						})
						.map(|(bi, _)| Hit::Block(ci, bi)),
				);
			}
		}
	}
	hits
}

pub async fn css_search(
	b: Arc<LoadedBundle>,
	query: &str,
	regex: bool,
	target: CssTarget,
	limit: usize,
) -> Result<String> {
	let css = b.css().await?;
	let matcher = matcher(query, regex)?;
	let hits = collect_hits(&css, &matcher, target);
	if hits.is_empty() {
		return Ok(format!("No css matches {query:?}"));
	}
	let mut out = format!(
		"{} css rule(s) match in build {}. Locations are chunk:line in \
		 get_css.\n",
		hits.len(),
		b.id
	);
	for hit in hits.iter().take(limit) {
		match *hit {
			Hit::Block(ci, bi) => {
				let chunk = &css.chunks[ci];
				render_block(&mut out, chunk, &chunk.blocks[bi]);
			}
			Hit::Line(ci, line) => {
				let chunk = &css.chunks[ci];
				let range = chunk
					.lines
					.range(chunk.code.len(), line, line);
				write!(out, "\n{}:{line}\n{}", chunk.name, &chunk.code[range])
					.unwrap();
			}
		}
	}
	if hits.len() > limit {
		write!(
			out,
			"\n[{} more not shown. Narrow the query or raise limit]",
			hits.len() - limit
		)
		.unwrap();
	}
	Ok(truncate(out, "Narrow the query or lower limit"))
}

pub async fn css_variables(
	b: Arc<LoadedBundle>,
	query: &str,
	limit: usize,
) -> Result<String> {
	let css = b.css().await?;
	let needle = query
		.trim()
		.trim_start_matches("--")
		.to_lowercase();
	let mut vars = css
		.vars
		.iter()
		.filter(|(name, _)| name.to_lowercase().contains(&needle))
		.collect::<Vec<_>>();
	if vars.is_empty() {
		return Ok(format!("No css variable matches {query:?}"));
	}
	vars.sort_unstable_by(|a, b| a.0.cmp(b.0));
	let mut out = format!("{} css variable(s) match\n", vars.len());
	for (name, var) in vars.iter().take(limit) {
		writeln!(
			out,
			"\n{name}: {} use(s), {} definition(s)",
			var.uses,
			var.defs.len()
		)
		.unwrap();
		for &(ci, bi, ref value) in var.defs.iter().take(6) {
			let chunk = &css.chunks[ci];
			let block = &chunk.blocks[bi];
			write!(out, "  {}", block.prelude).unwrap();
			if !block.context.is_empty() {
				write!(out, " (inside {})", block.context.join(" > ")).unwrap();
			}
			writeln!(
				out,
				": {value}  [{}:{}]",
				chunk.name,
				chunk.block_line(block)
			)
			.unwrap();
		}
		if var.defs.len() > 6 {
			writeln!(out, "  … {} more definitions", var.defs.len() - 6)
				.unwrap();
		}
	}
	if vars.len() > limit {
		write!(out, "\n[{} more not shown]", vars.len() - limit).unwrap();
	}
	Ok(truncate(out, "Use a more specific query or lower limit"))
}

pub async fn get_css(
	b: Arc<LoadedBundle>,
	chunk: Option<&str>,
	start_line: Option<usize>,
	end_line: Option<usize>,
) -> Result<String> {
	let css = b.css().await?;
	let Some(name) = chunk else {
		let mut chunks = css.chunks.iter().collect::<Vec<_>>();
		chunks.sort_unstable_by_key(|c| std::cmp::Reverse(c.raw_len));
		let mut out = format!(
			"{} css chunk(s) in build {}, largest first\n\n",
			chunks.len(),
			b.id
		);
		for c in chunks {
			writeln!(
				out,
				"{}: {} bytes, {} rules, {} lines formatted",
				c.name,
				c.raw_len,
				c.blocks.len(),
				c.lines.len()
			)
			.unwrap();
		}
		return Ok(truncate(out, "Too many chunks to list"));
	};
	let chunk = &css.chunks[css
		.chunk(name)
		.map_err(|e| anyhow!(e))?];
	let lines: &LineIndex = &chunk.lines;
	let total = lines.len();
	let start = start_line.unwrap_or(1).clamp(1, total);
	let end = end_line
		.unwrap_or(start + DEFAULT_LINES - 1)
		.clamp(start, total);
	let range = lines.range(chunk.code.len(), start, end);
	let mut out = format!(
		"/* {}, build {}, lines {start}-{end} of {total} (formatted) */\n",
		chunk.name, b.id
	);
	out.push_str(&number_lines(&chunk.code[range], start));
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::store::test_util::{bundle, bundle_with_css};

	const CSS: &[(&str, &str)] = &[
		(
			"aaa.css",
			".chatContent_f75fb0{display:flex;color:var(--text-normal)}\
			 @media (max-width:500px){.chatContent_f75fb0 .x{flex:1}}",
		),
		(
			"bbb.css",
			".theme-dark{--text-normal:#dbdee1}.theme-light{--text-normal:#313338}",
		),
	];

	#[tokio::test]
	async fn searches_selectors() {
		let b = bundle_with_css("abc", &[], CSS);
		let out = css_search(b, "chatContent", false, CssTarget::Selector, 10)
			.await
			.unwrap();
		assert!(out.starts_with("2 css rule(s)"), "{out}");
		assert!(out.contains("(inside @media (max-width:500px))"), "{out}");
	}

	#[tokio::test]
	async fn searches_anywhere_once_per_rule() {
		let b = bundle_with_css("abc", &[], CSS);
		let out = css_search(b, "flex", false, CssTarget::Any, 10)
			.await
			.unwrap();
		// `display:flex` and `flex:1` are in different rules
		assert!(out.starts_with("2 css rule(s)"), "{out}");
	}

	#[tokio::test]
	async fn lists_variables() {
		let b = bundle_with_css("abc", &[], CSS);
		let out = css_variables(b, "text-normal", 10)
			.await
			.unwrap();
		assert!(
			out.contains("--text-normal: 1 use(s), 2 definition(s)"),
			"{out}"
		);
		assert!(out.contains(".theme-dark: #dbdee1"), "{out}");
	}

	#[tokio::test]
	async fn reads_chunks() {
		let b = bundle_with_css("abc", &[], CSS);
		let list = get_css(b.clone(), None, None, None)
			.await
			.unwrap();
		assert!(list.contains("aaa.css"), "{list}");
		let out = get_css(b.clone(), Some("bb"), Some(2), Some(2))
			.await
			.unwrap();
		assert!(out.contains("    2    --text-normal:#dbdee1"), "{out}");
		assert!(
			get_css(b, Some("zzz"), None, None)
				.await
				.is_err()
		);
	}

	#[tokio::test]
	async fn no_css_is_an_error() {
		let b = bundle("abc", &[]);
		let err = css_search(b, "x", false, CssTarget::Any, 10)
			.await
			.unwrap_err();
		assert!(err.to_string().contains("has no css"), "{err}");
	}
}
