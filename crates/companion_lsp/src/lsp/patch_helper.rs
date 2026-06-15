//! Patch Helper — opens the post-patch source for a single Vencord patch in
//! a virtual editor view and keeps it in sync as the user edits the plugin
//! source.
//!
//! Flow:
//! 1. User invokes `vencord.openPatchHelper` (typically via the
//!    "Open in Patch Helper" code lens). The server resolves the patch from
//!    `{ uri, patchIndex }`, asks Discord for the un-patched module that
//!    satisfies the patch's `find`, applies the patch's replacements
//!    locally, formats the result, and ships a `vencord/patchHelper/open`
//!    request to the editor with the rendered content + an opaque patch id.
//! 2. Whenever the source document changes, we look up the helper by source
//!    URI, re-find the patch in the new AST (matched on find string +
//!    replacement count), re-apply, and push the new content via the
//!    `vencord/patchHelper/update` notification.
//! 3. On source close, the helper drops its entry.

use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result, anyhow, bail};
use dashmap::DashMap;
use oxc::{allocator::Allocator, ast::ast::RegExpFlags};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tower_lsp::{
	Client,
	lsp_types::{MessageType, Url, notification::Notification, request::Request},
};
use vencord_ast_parser::{Match, Patch, Replacer, VencordAstParser};

use crate::{
	discord_bridge::{
		messages::{ExtractModuleData, FindType, OutgoingKind},
		rpc::DEFAULT_TIMEOUT,
	},
	lsp::Backend,
	state::{Document, SharedState},
	vencord_ext,
};

#[derive(Debug)]
struct HelperEntry {
	id: String,
	plugin_name: String,
	/// Find expression as a single canonical string — string finds verbatim,
	/// regex finds kept as their pattern. Used as a fingerprint when
	/// relocating the patch after source edits.
	last_find: String,
	last_find_type: FindType,
	last_replacement_count: usize,
	/// Module Discord handed us. Cached so reapplies don't pay another
	/// extract round-trip if the source change didn't actually shift the
	/// find.
	module_source: String,
	module_number: i64,
	/// Most recently rendered (post-format) patched module. Diffed against
	/// the next render to compute the scroll-to-changes range — mirrors the
	/// `LastTwo<string>` ring buffer in the legacy TS helper.
	last_rendered: String,
}

#[derive(Default)]
pub struct Registry {
	counter: AtomicU64,
	by_source: DashMap<Url, Arc<Mutex<HelperEntry>>>,
}

impl Registry {
	fn next_id(&self) -> String {
		format!("ph-{:x}", self.counter.fetch_add(1, Ordering::Relaxed))
	}

	fn get(&self, src: &Url) -> Option<Arc<Mutex<HelperEntry>>> {
		self.by_source
			.get(src)
			.map(|e| e.value().clone())
	}

	fn close(&self, src: &Url) {
		self.by_source.remove(src);
	}
}

// ---------- public entrypoints ------------------------------------------

pub async fn open(backend: &Backend, args: Vec<Value>) -> Result<Value> {
	#[derive(Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct OpenArg {
		uri: String,
		patch_index: usize,
	}
	let first = args
		.first()
		.context("openPatchHelper requires {uri, patchIndex}")?;
	let arg: OpenArg = serde_json::from_value(first.clone())
		.context("expected {uri, patchIndex}")?;
	let url = Url::parse(&arg.uri)
		.with_context(|| format!("invalid uri {:?}", arg.uri))?;

	let doc = backend
		.state
		.get_document(&url)
		.with_context(|| format!("no open document for {url}"))?;

	let entry = get_or_insert(&backend.state, &url);

	let result =
		resolve_and_render(&backend.state, &doc, arg.patch_index, &entry).await;
	let (patch_id, module_content) = match result {
		Ok(v) => v,
		Err(e) => {
			// Don't leak a half-initialised entry if extract failed.
			backend.state.patch_helpers.close(&url);
			return Err(e);
		}
	};

	send_open(&backend.client, &url, &patch_id, &module_content).await?;
	Ok(json!({ "patchId": patch_id }))
}

/// Called from `document::on_did_change` after the buffer has been updated.
/// Re-resolves the patch in the new source, re-renders, and pushes new
/// content. Best-effort; logs and surfaces failures as toasts but never
/// panics.
pub async fn on_source_change(
	client: Client,
	state: SharedState,
	source_uri: Url,
) {
	let Some(entry) = state.patch_helpers.get(&source_uri) else {
		return;
	};
	let Some(doc) = state.get_document(&source_uri) else {
		return;
	};

	if let Err(e) = relocate_and_push(&state, &client, &doc, &source_uri, &entry).await {
		tracing::debug!(?e, %source_uri, "patch helper update failed");
		client
			.show_message(MessageType::WARNING, format!("PatchHelper: {e}"))
			.await;
		// Capture the id before we drop the registry entry so the editor
		// can close the matching virtual document.
		let patch_id = entry.lock().await.id.clone();
		state.patch_helpers.close(&source_uri);
		send_close(&client, &source_uri.to_string(), &patch_id).await;
	}
}

pub async fn on_source_close(client: &Client, state: &SharedState, source_uri: &Url) {
	// Grab the patch id before dropping the entry so the client knows
	// which virtual document to close.
	let patch_id = match state.patch_helpers.get(source_uri) {
		Some(arc) => arc.lock().await.id.clone(),
		None => return,
	};
	state.patch_helpers.close(source_uri);
	send_close(client, &source_uri.to_string(), &patch_id).await;
}

// ---------- internals ----------------------------------------------------

fn get_or_insert(state: &SharedState, url: &Url) -> Arc<Mutex<HelperEntry>> {
	if let Some(e) = state.patch_helpers.get(url) {
		return e;
	}
	let id = state.patch_helpers.next_id();
	let placeholder = HelperEntry {
		id,
		plugin_name: "MyPlugin".to_owned(),
		last_find: String::new(),
		last_find_type: FindType::String,
		last_replacement_count: 0,
		module_source: String::new(),
		module_number: 0,
		last_rendered: String::new(),
	};
	let arc = Arc::new(Mutex::new(placeholder));
	state
		.patch_helpers
		.by_source
		.insert(url.clone(), arc.clone());
	arc
}

async fn resolve_and_render(
	state: &SharedState,
	doc: &Document,
	patch_index: usize,
	entry: &Arc<Mutex<HelperEntry>>,
) -> Result<(String, String)> {
	let text = doc.text.clone();
	let extracted = tokio::task::spawn_blocking(move || {
		extract_patch(&text, patch_index)
	})
	.await
	.context("patch extraction task panicked")??;

	if !state.discord.is_connected().await {
		bail!("No Discord client connected.");
	}

	let module = fetch_module_for_patch(state, &extracted).await?;

	let patched = apply_patch_to_module(
		&module.module,
		&extracted.patch,
		&extracted.plugin_name,
	)
	.context("apply patch")?;
	let rendered = render_module(&patched, module.module_number);

	let mut guard = entry.lock().await;
	guard.plugin_name = extracted.plugin_name;
	guard.last_find = extracted.find_string;
	guard.last_find_type = extracted.find_type;
	guard.last_replacement_count = extracted.patch.replacement.len();
	guard.module_source = module.module;
	guard.module_number = module.module_number;
	guard.last_rendered = rendered.clone();
	Ok((guard.id.clone(), rendered))
}

async fn relocate_and_push(
	state: &SharedState,
	client: &Client,
	doc: &Document,
	source_uri: &Url,
	entry: &Arc<Mutex<HelperEntry>>,
) -> Result<()> {
	let (last_find, find_type, last_rcount) = {
		let g = entry.lock().await;
		(
			g.last_find.clone(),
			g.last_find_type,
			g.last_replacement_count,
		)
	};

	let text = doc.text.clone();
	let extracted = tokio::task::spawn_blocking(move || {
		relocate_patch(&text, &last_find, find_type, last_rcount)
	})
	.await
	.context("patch relocate task panicked")??;

	// If the find moved, re-extract the module against the new find;
	// otherwise reuse the cached source.
	let need_reextract = {
		let g = entry.lock().await;
		extracted.find_string != g.last_find || g.module_source.is_empty()
	};
	let (module_source, module_number) = if need_reextract {
		if !state.discord.is_connected().await {
			bail!("No Discord client connected.");
		}
		let m = fetch_module_for_patch(state, &extracted).await?;
		(m.module, m.module_number)
	} else {
		let g = entry.lock().await;
		(g.module_source.clone(), g.module_number)
	};

	let patched = apply_patch_to_module(
		&module_source,
		&extracted.patch,
		&extracted.plugin_name,
	)?;
	let rendered = render_module(&patched, module_number);

	let (patch_id, source_uri_str, reveal) = {
		let mut g = entry.lock().await;
		let reveal = changed_line_range(&g.last_rendered, &rendered);
		g.plugin_name = extracted.plugin_name;
		g.last_find = extracted.find_string;
		g.last_find_type = extracted.find_type;
		g.last_replacement_count = extracted.patch.replacement.len();
		g.module_source = module_source;
		g.module_number = module_number;
		g.last_rendered = rendered.clone();
		(g.id.clone(), source_uri.to_string(), reveal)
	};

	send_update(client, &source_uri_str, &patch_id, &rendered, reveal).await;
	Ok(())
}

/// Find the line range in `new` that differs from `old`.
///
/// Matches what the legacy TS helper used `fast-diff` for: take the common
/// prefix and common suffix, then center on whatever's in between. Returns
/// `None` if the two strings are identical or `old` is empty (e.g. the very
/// first update after open — no previous content to diff against).
fn changed_line_range(old: &str, new: &str) -> Option<(u32, u32)> {
	if old.is_empty() || old == new {
		return None;
	}
	let old_b = old.as_bytes();
	let new_b = new.as_bytes();
	let max = old_b.len().min(new_b.len());

	let mut prefix = 0;
	while prefix < max && old_b[prefix] == new_b[prefix] {
		prefix += 1;
	}
	// Back off if we landed mid–UTF-8 codepoint.
	while prefix > 0 && !new.is_char_boundary(prefix) {
		prefix -= 1;
	}

	let suffix_budget = max - prefix;
	let mut suffix = 0;
	while suffix < suffix_budget
		&& old_b[old_b.len() - 1 - suffix] == new_b[new_b.len() - 1 - suffix]
	{
		suffix += 1;
	}
	while suffix > 0 && !new.is_char_boundary(new_b.len() - suffix) {
		suffix -= 1;
	}

	let start = prefix;
	let end = new_b.len().saturating_sub(suffix).max(start);
	let start_line = byte_to_line(new, start);
	let end_line = byte_to_line(new, end);
	Some((start_line, end_line))
}

fn byte_to_line(s: &str, byte: usize) -> u32 {
	let cap = byte.min(s.len());
	s.as_bytes()[..cap]
		.iter()
		.filter(|&&b| b == b'\n')
		.count() as u32
}

#[derive(Debug)]
struct ExtractedPatch {
	patch: Patch,
	plugin_name: String,
	find_string: String,
	find_type: FindType,
}

fn extract_patch(source: &str, patch_index: usize) -> Result<ExtractedPatch> {
	let alloc = Allocator::default();
	let parser = VencordAstParser::try_new(&alloc, source, None)
		.map_err(|e| anyhow!("parse plugin source: {e}"))?;
	let plugin_name = parser
		.plugin_info()
		.context("source does not look like a Vencord plugin")?
		.name
		.to_owned();
	let mut patches = parser
		.patches(true)
		.map_err(|e| anyhow!("canonicalize patches: {e}"))?;
	if patch_index >= patches.len() {
		bail!(
			"patch index {patch_index} out of range ({} patches found)",
			patches.len(),
		);
	}
	let patch = patches.swap_remove(patch_index);
	let (find_type, find_string) = find_signature(&patch);
	Ok(ExtractedPatch {
		patch,
		plugin_name,
		find_string,
		find_type,
	})
}

fn relocate_patch(
	source: &str,
	target_find: &str,
	target_find_type: FindType,
	target_rcount: usize,
) -> Result<ExtractedPatch> {
	let alloc = Allocator::default();
	let parser = VencordAstParser::try_new(&alloc, source, None)
		.map_err(|e| anyhow!("parse plugin source: {e}"))?;
	let plugin_name = parser
		.plugin_info()
		.context("source no longer looks like a Vencord plugin")?
		.name
		.to_owned();
	let patches = parser
		.patches(true)
		.map_err(|e| anyhow!("canonicalize patches: {e}"))?;

	let mut best: Option<Patch> = None;
	let mut close: Option<Patch> = None;
	for p in patches {
		let (t, s) = find_signature(&p);
		let same_type = matches!(
			(t, target_find_type),
			(FindType::String, FindType::String)
				| (FindType::Regex, FindType::Regex),
		);
		if !same_type || s != target_find {
			continue;
		}
		if p.replacement.len() == target_rcount {
			best = Some(p);
			break;
		}
		if close.is_none() && p.replacement.len().abs_diff(target_rcount) < 2 {
			close = Some(p);
		}
	}

	let patch = best.or(close).context(
		"lost patch — find no longer matches any patch in the source",
	)?;
	let (find_type, find_string) = find_signature(&patch);
	Ok(ExtractedPatch {
		patch,
		plugin_name,
		find_string,
		find_type,
	})
}

fn find_signature(patch: &Patch) -> (FindType, String) {
	match &patch.find.v {
		Match::Str(finder) => (
			FindType::String,
			std::str::from_utf8(finder.needle())
				.unwrap_or("")
				.to_owned(),
		),
		Match::Regex(mr) => (FindType::Regex, mr.pattern.clone()),
	}
}

struct ModuleFetch {
	module: String,
	module_number: i64,
}

async fn fetch_module_for_patch(
	state: &SharedState,
	extracted: &ExtractedPatch,
) -> Result<ModuleFetch> {
	let sender = state.discord.sender().await;
	let frame = sender
		.request(
			OutgoingKind::Extract {
				data: json!({
					"extractType": "search",
					"findType":    find_type_kind(extracted.find_type),
					"usePatched":  false,
					"idOrSearch":  &extracted.find_string,
				}),
			},
			DEFAULT_TIMEOUT,
		)
		.await?;
	let payload: ExtractModuleData = frame.parse_data()?;
	Ok(ModuleFetch {
		module: payload.module,
		module_number: payload.module_number,
	})
}

const fn find_type_kind(t: FindType) -> &'static str {
	match t {
		FindType::String => "string",
		FindType::Regex => "regex",
	}
}

/// Apply every replacement in `patch` to `module` and return the formatted
/// post-patch text.
///
/// Happy path: splice the replacements straight into the original module
/// (spans are in original coordinates) and format the result. When the patch
/// yields valid JS — the common case — this formats the whole module,
/// replacement text included.
///
/// Fallback (mirrors `ReporterState::format_syntax_error`): if the patched
/// module doesn't parse — because the replacement text itself produces invalid
/// JS — format the **unpatched** module instead (it always parses), map each
/// replacement's span into the formatted code, and splice the raw replacement
/// text in. The replacement text stays unformatted, but the surrounding module
/// is still cleanly formatted.
///
/// As with the reporter, replacements are computed against the **original**
/// module rather than cumulatively against each other's output.
fn apply_patch_to_module(
	module: &str,
	patch: &Patch,
	plugin_name: &str,
) -> Result<String> {
	let mut ranges: Vec<(usize, usize, String)> = Vec::new();
	for (i, repl) in patch.replacement.iter().enumerate() {
		collect_replacement_ranges(module, repl, plugin_name, &mut ranges)
			.with_context(|| format!("replacement {} failed", i + 1))?;
	}

	// Splice into the original module first. Offsets are already in original
	// coordinates, so no mapping is needed and the result is logically exact.
	let patched = splice_ranges(module, ranges.clone());
	let alloc = Allocator::default();
	if let Ok(formatted) = pretty_printer::format_with_alloc(&patched, &alloc, 4) {
		return Ok(formatted.code);
	}

	// Patched module won't parse — fall back to formatting the unpatched
	// module and splicing the raw replacement text at mapped positions.
	let alloc = Allocator::default();
	let Ok(pretty_printer::FormattedContent { code, mappings }) =
		pretty_printer::format_with_alloc(module, &alloc, 4)
	else {
		// Module didn't parse on its own either — extremely unusual for a
		// Discord webpack module. Return the raw patched text so the rest of
		// the helper still works.
		return Ok(patched);
	};

	let mapped = ranges
		.into_iter()
		.map(|(start, end, txt)| {
			let (ns, ne) = map_span(&mappings, start as u32, end as u32);
			(ns as usize, ne as usize, txt)
		})
		.collect();
	Ok(splice_ranges(&code, mapped))
}

/// Apply `(start, end, replacement_text)` byte-span splices to `code`.
/// Splices run back-to-front so earlier offsets stay valid as later ones are
/// removed; out-of-bounds spans are clamped.
fn splice_ranges(code: &str, mut ranges: Vec<(usize, usize, String)>) -> String {
	let mut out = code.to_owned();
	ranges.sort_by_key(|(start, _, _)| *start);
	while let Some((start, end, txt)) = ranges.pop() {
		let start = start.min(out.len());
		let end = end.min(out.len()).max(start);
		out.replace_range(start..end, &txt);
	}
	out
}

/// Push `(start, end, replacement_text)` tuples for `repl`'s matches in
/// `original` onto `ranges`. Spans are byte offsets into `original`.
///
/// A replacement that matches nothing is treated as a no-op (no ranges
/// pushed), not an error: the patch helper is a live preview that re-renders
/// on every keystroke, so a transiently non-matching replacement must not
/// stop the module from opening. Genuine failures (bad regex, non-utf8 find,
/// unsupported replacer shape) still propagate.
fn collect_replacement_ranges(
	original: &str,
	repl: &vencord_ast_parser::Replacement,
	plugin_name: &str,
	ranges: &mut Vec<(usize, usize, String)>,
) -> Result<()> {
	match &repl.match_.v {
		Match::Str(finder) => {
			let needle = std::str::from_utf8(finder.needle())
				.context("non-utf8 find string")?;
			let Some(start) = original.find(needle) else {
				tracing::debug!(needle, "patch helper: replacement had no effect");
				return Ok(());
			};
			// A string match exposes no capture groups, so there's nothing for
			// a template's interpolations to bind to. Only literal string
			// replacements make sense here.
			let Replacer::Str(s) = &repl.replace.v else {
				bail!("template replace requires a regex match");
			};
			ranges.push((
				start,
				start + needle.len(),
				substitute_self(s, plugin_name),
			));
			Ok(())
		}
		Match::Regex(mr) => {
			let flags = mr.flags;
			let regex = regress::Regex::with_flags(
				&mr.pattern,
				regress::Flags {
					icase: flags.contains(RegExpFlags::I),
					multiline: flags.contains(RegExpFlags::M),
					dot_all: flags.contains(RegExpFlags::S),
					unicode: flags.contains(RegExpFlags::U),
					unicode_sets: flags.contains(RegExpFlags::V),
					no_opt: false,
				},
			)
			.map_err(|e| anyhow!("compile match regex: {e}"))?;
			let matches: Vec<regress::Match> = if flags.contains(RegExpFlags::G) {
				regex.find_iter(original).collect()
			} else {
				regex.find(original).into_iter().collect()
			};
			if matches.is_empty() {
				tracing::debug!(
					pattern = %mr.pattern,
					"patch helper: replacement had no effect",
				);
				return Ok(());
			}
			for m in &matches {
				let txt = render_replacement(&repl.replace.v, original, m, plugin_name)?;
				ranges.push((m.start(), m.end(), txt));
			}
			Ok(())
		}
	}
}

/// Evaluate one replacement against regex match `m` and resolve `$self`.
///
/// Both string (`"$1.foo"`) and template (`` (_, a) => `${a}.foo` ``)
/// replacements are evaluated by the parser's [`Replacer::do_replace`], which
/// handles `$n`/`$<name>` group interpolation for strings and parameter→group
/// binding for templates. We then resolve `$self` over the result, so the
/// token works in either form (e.g. the `$self.setShift(...)` literal inside a
/// template arrow function).
///
/// `do_replace` panics if a template interpolates a capture group the match
/// regex doesn't define — the parser doesn't validate template captures the
/// way it does string `$n` refs — so we guard it and surface the failure as a
/// normal error rather than tearing down the helper task.
fn render_replacement(
	replacer: &Replacer,
	original: &str,
	m: &regress::Match,
	plugin_name: &str,
) -> Result<String> {
	let raw = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		replacer.do_replace(original, m)
	}))
	.map_err(|_| {
		anyhow!("replacement references a capture group the match does not define")
	})?;
	Ok(substitute_self(&raw, plugin_name))
}

/// Translate an `[original_start, original_end)` byte span into the
/// formatted code's coordinates using the `(original_pos, formatted_pos)`
/// mappings emitted by `pretty_printer::format_with_alloc`. Mappings are
/// sorted ascending by original position; the largest entry with
/// `before <= orig` gives the right offset (between two consecutive mappings
/// the text is byte-identical, so we add the delta).
fn map_span(mappings: &[(u32, u32)], start: u32, end: u32) -> (u32, u32) {
	(map_pos(mappings, start), map_pos(mappings, end))
}

fn map_pos(mappings: &[(u32, u32)], orig: u32) -> u32 {
	for &(before, after) in mappings.iter().rev() {
		if orig >= before {
			return after + (orig - before);
		}
	}
	0
}

fn substitute_self(s: &str, plugin_name: &str) -> String {
	// Mirrors `Vencord.Plugins.plugins[JSON.stringify(name)]` from the TS
	// patch helper — JSON encoding handles names with quotes/escapes.
	let key = serde_json::to_string(plugin_name)
		.unwrap_or_else(|_| format!("\"{plugin_name}\""));
	let self_value = format!("Vencord.Plugins.plugins[{key}]");
	s.replace("$self", &self_value)
}

fn render_module(code: &str, module_number: i64) -> String {
	use std::fmt::Write as _;
	if code.starts_with("// Webpack Module") {
		return code.to_owned();
	}
	let mut out = String::with_capacity(code.len() + 64);
	let _ = writeln!(out, "// Webpack Module {module_number}");
	let _ = writeln!(out, "0,");
	out.push_str(code);
	out
}

// ---------- transport ----------------------------------------------------

async fn send_open(
	client: &Client,
	source_uri: &Url,
	patch_id: &str,
	module_content: &str,
) -> Result<()> {
	struct PatchHelperOpen;
	impl Request for PatchHelperOpen {
		type Params = Value;
		type Result = Value;
		const METHOD: &'static str = vencord_ext::PATCH_HELPER_OPEN_METHOD;
	}
	client
		.send_request::<PatchHelperOpen>(json!({
			"sourceUri":     source_uri.to_string(),
			"patchId":       patch_id,
			"moduleContent": module_content,
		}))
		.await
		.map(drop)
		.map_err(|e| {
			anyhow!("editor does not support vencord/patchHelper/open: {e}")
		})
}

async fn send_close(client: &Client, source_uri: &str, patch_id: &str) {
	struct PatchHelperClose;
	impl Notification for PatchHelperClose {
		type Params = Value;
		const METHOD: &'static str = vencord_ext::PATCH_HELPER_CLOSE_METHOD;
	}
	client
		.send_notification::<PatchHelperClose>(json!({
			"sourceUri": source_uri,
			"patchId":   patch_id,
		}))
		.await;
}

async fn send_update(
	client: &Client,
	source_uri: &str,
	patch_id: &str,
	module_content: &str,
	reveal: Option<(u32, u32)>,
) {
	struct PatchHelperUpdate;
	impl Notification for PatchHelperUpdate {
		type Params = Value;
		const METHOD: &'static str = vencord_ext::PATCH_HELPER_UPDATE_METHOD;
	}
	let reveal_range = reveal.map(|(s, e)| json!({ "startLine": s, "endLine": e }));
	client
		.send_notification::<PatchHelperUpdate>(json!({
			"sourceUri":     source_uri,
			"patchId":       patch_id,
			"moduleContent": module_content,
			"revealRange":   reveal_range,
		}))
		.await;
}

#[cfg(test)]
mod tests {
	use super::*;

	const PLUGIN: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "MyPlugin",
    patches: [{
        find: "foo.bar",
        replacement: { match: /(foo)/, replace: "$1.baz" }
    }],
});
"#;

	#[test]
	fn extract_picks_patch_by_index() {
		let e = extract_patch(PLUGIN, 0).unwrap();
		assert_eq!(e.plugin_name, "MyPlugin");
		assert_eq!(e.find_string, "foo.bar");
		assert!(matches!(e.find_type, FindType::String));
		assert_eq!(e.patch.replacement.len(), 1);
	}

	#[test]
	fn extract_rejects_out_of_range() {
		let err = extract_patch(PLUGIN, 5).unwrap_err();
		assert!(err.to_string().contains("out of range"));
	}

	#[test]
	fn apply_patch_runs_regex_replacement() {
		let e = extract_patch(PLUGIN, 0).unwrap();
		let result =
			apply_patch_to_module("var foo = 1; foo.bar", &e.patch, "MyPlugin")
				.unwrap();
		assert!(result.contains("foo.baz"));
	}

	#[test]
	fn apply_patch_substitutes_self_token() {
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "MyPlugin",
    patches: [{
        find: "x",
        replacement: { match: /x/, replace: "$self.go()" }
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		let result =
			apply_patch_to_module("var x = 1; x;", &e.patch, "MyPlugin").unwrap();
		assert!(
			result.contains("Vencord.Plugins.plugins[\"MyPlugin\"].go()"),
			"got: {result}",
		);
	}

	#[test]
	fn relocate_finds_renamed_replacement_count() {
		const ORIG: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "stable",
        replacement: [
            { match: /a/, replace: "b" },
            { match: /c/, replace: "d" }
        ]
    }],
});
"#;
		// Same find, one replacement (delta of 1 -> close match accepted).
		const CHANGED: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "stable",
        replacement: { match: /a/, replace: "b" }
    }],
});
"#;
		let _orig = extract_patch(ORIG, 0).unwrap();
		let relocated =
			relocate_patch(CHANGED, "stable", FindType::String, 2).unwrap();
		assert_eq!(relocated.find_string, "stable");
		assert_eq!(relocated.patch.replacement.len(), 1);
	}

	#[test]
	fn render_module_adds_webpack_header_when_missing() {
		let rendered = render_module("var x = 1;", 12345);
		assert!(rendered.starts_with("// Webpack Module 12345"));
	}

	#[test]
	fn apply_patch_keeps_formatting_when_replacement_breaks_syntax() {
		// Replacement substitutes a fragment that produces invalid JS — the
		// surrounding code must still come back formatted (newlines + indent).
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "x",
        replacement: { match: /var x = 1;/, replace: "var x =" }
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		let module = "function a(){var x = 1;var y = 2;}";
		let result =
			apply_patch_to_module(module, &e.patch, "P").unwrap();
		// Replacement landed.
		assert!(
			result.contains("var x ="),
			"replacement missing in:\n{result}",
		);
		assert!(
			!result.contains("var x = 1;"),
			"original line should be gone:\n{result}",
		);
		// Formatting kicked in: the formatter splits a one-liner function
		// body across multiple lines.
		assert!(
			result.lines().count() > 1,
			"output should be multi-line (was: {result:?})",
		);
	}

	#[test]
	fn changed_line_range_finds_single_edit() {
		let old = "line1\nline2\nline3\nline4\n";
		let new = "line1\nLINE2\nline3\nline4\n";
		let (start, end) = changed_line_range(old, new).unwrap();
		assert_eq!(start, 1);
		assert_eq!(end, 1);
	}

	#[test]
	fn changed_line_range_spans_multiple_lines() {
		let old = "a\nb\nc\nd\n";
		let new = "a\nB\nC\nd\n";
		let (start, end) = changed_line_range(old, new).unwrap();
		assert_eq!(start, 1);
		assert_eq!(end, 2);
	}

	#[test]
	fn changed_line_range_returns_none_for_identical_or_empty() {
		assert!(changed_line_range("", "anything").is_none());
		assert!(changed_line_range("same", "same").is_none());
	}

	#[test]
	fn apply_patch_evaluates_template_replacement() {
		// Arrow-function replace returning a template literal: groups are bound
		// by parameter position and `$self` appears inside a literal segment.
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "x",
        replacement: { match: /(a)(b)/, replace: (_, first, second) => `${first}$self.go(${second})` }
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		assert!(matches!(e.patch.replacement[0].replace.v, Replacer::Template(_)));
		let result =
			apply_patch_to_module("var z = ab;", &e.patch, "P").unwrap();
		assert!(
			result.contains("aVencord.Plugins.plugins[\"P\"].go(b)"),
			"template not evaluated: {result}",
		);
	}

	#[test]
	fn apply_patch_rejects_template_with_string_match() {
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "x",
        replacement: { match: "ab", replace: (m) => `${m}!` }
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		let err = apply_patch_to_module("var z = ab;", &e.patch, "P")
			.unwrap_err();
		assert!(
			err.to_string().contains("requires a regex match")
				|| format!("{err:#}").contains("requires a regex match"),
			"unexpected error: {err:#}",
		);
	}

	#[test]
	fn apply_patch_formats_replacement_text_when_valid() {
		// When the patched module is valid JS, the spliced-in replacement text
		// must be formatted too — not left in its raw, unformatted shape.
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "x",
        replacement: { match: /return 1;/, replace: "if(y){return 2;}" }
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		let result =
			apply_patch_to_module("function a(){return 1;}", &e.patch, "P")
				.unwrap();
		// Formatter normalizes the raw `if(y)` into `if (y)`.
		assert!(
			result.contains("if (y)"),
			"replacement text not formatted:\n{result}",
		);
	}

	#[test]
	fn apply_patch_skips_replacement_with_no_effect() {
		// The match never appears in the module — the helper must still render
		// (returning the formatted module) instead of failing to open.
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "x",
        replacement: { match: /never-matches-anything/, replace: "boom" }
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		let result =
			apply_patch_to_module("var x = 1;", &e.patch, "P").unwrap();
		assert!(result.contains("var x = 1;"), "got: {result}");
		assert!(!result.contains("boom"), "got: {result}");
	}

	#[test]
	fn apply_patch_applies_matching_replacements_around_a_no_op() {
		// One replacement matches, one doesn't. The matching one still lands.
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "x",
        replacement: [
            { match: /nope/, replace: "boom" },
            { match: /(foo)/, replace: "$1.baz" }
        ]
    }],
});
"#;
		let e = extract_patch(SRC, 0).unwrap();
		let result =
			apply_patch_to_module("var foo = 1;", &e.patch, "P").unwrap();
		assert!(result.contains("foo.baz"), "got: {result}");
		assert!(!result.contains("boom"), "got: {result}");
	}

	#[test]
	fn render_module_keeps_existing_header() {
		let src = "// Webpack Module 9\n0,\nvar x = 1;\n";
		let rendered = render_module(src, 9);
		assert_eq!(
			rendered.matches("// Webpack Module").count(),
			1,
			"should not double-emit the header",
		);
	}
}
