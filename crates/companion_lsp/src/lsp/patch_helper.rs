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
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
	plugin_name: String,
	/// Find expression as a single canonical string — string finds verbatim,
	/// regex finds kept as their pattern. Used as a fingerprint when
	/// relocating the patch after source edits.
	last_find: String,
	last_find_type: FindType,
	/// Which patch sharing this find this helper tracks: 0 = the first patch
	/// in source order whose find matches `last_find`, 1 = the second, and so
	/// on. Two patches that resolve to the same webpack module have identical
	/// finds, so the find alone can't tell them apart — the occurrence index
	/// keeps each helper pinned to its own patch.
	occurrence: usize,
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

impl HelperEntry {
	fn placeholder() -> Self {
		Self {
			plugin_name: "MyPlugin".to_owned(),
			last_find: String::new(),
			last_find_type: FindType::String,
			occurrence: 0,
			module_source: String::new(),
			module_number: 0,
			last_rendered: String::new(),
		}
	}
}

/// A single live patch-helper session. The `id` lives outside the async
/// `Mutex` so the registry can match and remove handles without awaiting a
/// lock.
#[derive(Clone)]
struct HelperHandle {
	id: String,
	entry: Arc<Mutex<HelperEntry>>,
}

#[derive(Default)]
pub struct Registry {
	counter: AtomicU64,
	/// Every helper open for a given plugin source. A source can drive several
	/// helpers at once (one per patch), so this is a list, not a single slot —
	/// keying by source alone would make opening a second patch clobber the
	/// first.
	by_source: DashMap<Url, Vec<HelperHandle>>,
}

impl Registry {
	fn next_id(&self) -> String {
		format!("ph-{:x}", self.counter.fetch_add(1, Ordering::Relaxed))
	}

	fn get_all(&self, src: &Url) -> Vec<HelperHandle> {
		self.by_source
			.get(src)
			.map(|e| e.value().clone())
			.unwrap_or_default()
	}

	fn push(&self, src: &Url, handle: HelperHandle) {
		self.by_source
			.entry(src.clone())
			.or_default()
			.push(handle);
	}

	/// Drop one helper by id, removing the source key entirely once its last
	/// helper is gone.
	fn remove_entry(&self, src: &Url, id: &str) {
		let mut now_empty = false;
		if let Some(mut e) = self.by_source.get_mut(src) {
			e.value_mut().retain(|h| h.id != id);
			now_empty = e.value().is_empty();
		}
		// The get_mut guard is dropped above before remove() so we don't
		// re-lock the same shard.
		if now_empty {
			self.by_source.remove(src);
		}
	}

	/// Remove and return every helper for a source (used on source close).
	fn take_all(&self, src: &Url) -> Vec<HelperHandle> {
		self.by_source
			.remove(src)
			.map(|(_, v)| v)
			.unwrap_or_default()
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

	// Resolve the patch up front (cheap, no Discord round-trip) so we can key
	// the helper by the patch's identity — find + occurrence — rather than by
	// the source URI alone.
	let text = doc.text.clone();
	let extracted = tokio::task::spawn_blocking(move || {
		extract_patch(&text, arg.patch_index)
	})
	.await
	.context("patch extraction task panicked")??;

	// Re-invoking the lens on a patch that already has a helper open should
	// refocus that helper, not spawn a duplicate. Anything else — including a
	// different patch that resolves to the *same* webpack module — gets its
	// own helper.
	let (handle, freshly_created) = if let Some(h) =
		find_existing(&backend.state, &url, &extracted).await
	{
		(h, false)
	} else {
		let h = HelperHandle {
			id: backend.state.patch_helpers.next_id(),
			entry: Arc::new(Mutex::new(HelperEntry::placeholder())),
		};
		backend.state.patch_helpers.push(&url, h.clone());
		(h, true)
	};

	let module_content = match render_into(&backend.state, extracted, &handle).await {
		Ok(v) => v,
		Err(e) => {
			// Don't leak a half-initialised entry if extract/fetch failed —
			// but leave an already-working helper alone if a re-open hit a
			// transient error (e.g. Discord briefly disconnected).
			if freshly_created {
				backend.state.patch_helpers.remove_entry(&url, &handle.id);
			}
			return Err(e);
		}
	};

	send_open(&backend.client, &url, &handle.id, &module_content).await?;
	Ok(serde_json::to_value(PatchIdResponse {
		patch_id: handle.id,
	})?)
}

/// `{ patchId }` — returned to the editor from `openPatchHelper`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchIdResponse {
	patch_id: String,
}

/// Find a helper already tracking this exact patch (same find + occurrence),
/// so a repeat "Open in Patch Helper" refocuses instead of duplicating.
async fn find_existing(
	state: &SharedState,
	url: &Url,
	extracted: &ExtractedPatch,
) -> Option<HelperHandle> {
	for h in state.patch_helpers.get_all(url) {
		let g = h.entry.lock().await;
		if same_find_type(g.last_find_type, extracted.find_type)
			&& g.last_find == extracted.find_string
			&& g.occurrence == extracted.occurrence
		{
			drop(g);
			return Some(h);
		}
	}
	None
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
	let handles = state.patch_helpers.get_all(&source_uri);
	if handles.is_empty() {
		return;
	}
	let Some(doc) = state.get_document(&source_uri) else {
		return;
	};

	// Each helper relocates its own patch independently, so two helpers
	// tracking patches with the same find (i.e. the same webpack module) stay
	// pinned to their respective occurrences.
	for handle in handles {
		if let Err(e) =
			relocate_and_push(&state, &client, &doc, &source_uri, &handle).await
		{
			tracing::debug!(?e, %source_uri, id = %handle.id, "patch helper update failed");
			client
				.show_message(MessageType::WARNING, format!("PatchHelper: {e}"))
				.await;
			state.patch_helpers.remove_entry(&source_uri, &handle.id);
			send_close(&client, &source_uri.to_string(), &handle.id).await;
		}
	}
}

pub async fn on_source_close(client: &Client, state: &SharedState, source_uri: &Url) {
	// Drop every helper tracking this source and tell the editor to close each
	// matching virtual document.
	for handle in state.patch_helpers.take_all(source_uri) {
		send_close(client, &source_uri.to_string(), &handle.id).await;
	}
}

// ---------- internals ----------------------------------------------------

/// Fetch the patch's module from Discord, apply the patch, render, and store
/// the result into `handle`. Returns the rendered module text. Shared by the
/// open path (which then sends `open`) — the source-change path goes through
/// `relocate_and_push` instead, which additionally computes the reveal range.
async fn render_into(
	state: &SharedState,
	extracted: ExtractedPatch,
	handle: &HelperHandle,
) -> Result<String> {
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

	let mut guard = handle.entry.lock().await;
	guard.plugin_name = extracted.plugin_name;
	guard.last_find = extracted.find_string;
	guard.last_find_type = extracted.find_type;
	guard.occurrence = extracted.occurrence;
	guard.module_source = module.module;
	guard.module_number = module.module_number;
	guard.last_rendered = rendered.clone();
	Ok(rendered)
}

async fn relocate_and_push(
	state: &SharedState,
	client: &Client,
	doc: &Document,
	source_uri: &Url,
	handle: &HelperHandle,
) -> Result<()> {
	let (last_find, find_type, occurrence) = {
		let g = handle.entry.lock().await;
		(g.last_find.clone(), g.last_find_type, g.occurrence)
	};

	let text = doc.text.clone();
	let extracted = tokio::task::spawn_blocking(move || {
		relocate_patch(&text, &last_find, find_type, occurrence)
	})
	.await
	.context("patch relocate task panicked")??;

	// If the find moved, re-extract the module against the new find;
	// otherwise reuse the cached source.
	let need_reextract = {
		let g = handle.entry.lock().await;
		extracted.find_string != g.last_find || g.module_source.is_empty()
	};
	let (module_source, module_number) = if need_reextract {
		if !state.discord.is_connected().await {
			bail!("No Discord client connected.");
		}
		let m = fetch_module_for_patch(state, &extracted).await?;
		(m.module, m.module_number)
	} else {
		let g = handle.entry.lock().await;
		(g.module_source.clone(), g.module_number)
	};

	let patched = apply_patch_to_module(
		&module_source,
		&extracted.patch,
		&extracted.plugin_name,
	)?;
	let rendered = render_module(&patched, module_number);

	let (source_uri_str, reveal) = {
		let mut g = handle.entry.lock().await;
		let reveal = changed_line_range(&g.last_rendered, &rendered);
		g.plugin_name = extracted.plugin_name;
		g.last_find = extracted.find_string;
		g.last_find_type = extracted.find_type;
		g.occurrence = extracted.occurrence;
		g.module_source = module_source;
		g.module_number = module_number;
		g.last_rendered = rendered.clone();
		(source_uri.to_string(), reveal)
	};

	send_update(client, &source_uri_str, &handle.id, &rendered, reveal).await;
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
	/// Position of this patch among all patches that share its find — see
	/// [`HelperEntry::occurrence`].
	occurrence: usize,
}

/// Two `FindType`s describe the same kind of find (string vs regex).
fn same_find_type(a: FindType, b: FindType) -> bool {
	matches!(
		(a, b),
		(FindType::String, FindType::String) | (FindType::Regex, FindType::Regex),
	)
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
	let (find_type, find_string) = find_signature(&patches[patch_index]);
	// Count earlier patches sharing this find so two patches that resolve to
	// the same module remain distinguishable by their order in the source.
	let occurrence = patches[..patch_index]
		.iter()
		.filter(|p| {
			let (t, s) = find_signature(p);
			same_find_type(t, find_type) && s == find_string
		})
		.count();
	let patch = patches.swap_remove(patch_index);
	Ok(ExtractedPatch {
		patch,
		plugin_name,
		find_string,
		find_type,
		occurrence,
	})
}

fn relocate_patch(
	source: &str,
	target_find: &str,
	target_find_type: FindType,
	target_occurrence: usize,
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

	// Walk the patches that share this find in source order and pick the one
	// at the tracked occurrence. If the source changed so that occurrence no
	// longer exists (e.g. one of two identical-find patches was deleted),
	// clamp to the last patch that still matches rather than dropping the
	// helper entirely.
	let mut seen = 0usize;
	let mut chosen: Option<Patch> = None;
	let mut last_match: Option<Patch> = None;
	for p in patches {
		let (t, s) = find_signature(&p);
		if !same_find_type(t, target_find_type) || s != target_find {
			continue;
		}
		if seen == target_occurrence {
			chosen = Some(p);
			break;
		}
		seen += 1;
		last_match = Some(p);
	}

	let (patch, occurrence) = match chosen {
		Some(p) => (p, target_occurrence),
		None => {
			let p = last_match.context(
				"lost patch — find no longer matches any patch in the source",
			)?;
			(p, seen.saturating_sub(1))
		}
	};
	let (find_type, find_string) = find_signature(&patch);
	Ok(ExtractedPatch {
		patch,
		plugin_name,
		find_string,
		find_type,
		occurrence,
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
				data: serde_json::to_value(ExtractSearchRequest {
					extract_type: "search",
					find_type: find_type_kind(extracted.find_type),
					use_patched: false,
					id_or_search: &extracted.find_string,
				})?,
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

/// `{ extractType: "search", findType, usePatched, idOrSearch }` — asks the
/// Discord bridge to find the module satisfying a patch's `find`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractSearchRequest<'a> {
	extract_type: &'static str,
	find_type: &'static str,
	use_patched: bool,
	id_or_search: &'a str,
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

/// `{ startLine, endLine }` — the editor scrolls this range into view.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevealRange {
	start_line: u32,
	end_line: u32,
}

/// Params for the `vencord/patchHelper/open` request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchHelperOpenParams {
	source_uri: String,
	patch_id: String,
	module_content: String,
}

/// Params for the `vencord/patchHelper/close` notification.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchHelperCloseParams {
	source_uri: String,
	patch_id: String,
}

/// Params for the `vencord/patchHelper/update` notification.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchHelperUpdateParams {
	source_uri: String,
	patch_id: String,
	module_content: String,
	reveal_range: Option<RevealRange>,
}

async fn send_open(
	client: &Client,
	source_uri: &Url,
	patch_id: &str,
	module_content: &str,
) -> Result<()> {
	struct PatchHelperOpen;
	impl Request for PatchHelperOpen {
		type Params = PatchHelperOpenParams;
		type Result = Value;
		const METHOD: &'static str = vencord_ext::PATCH_HELPER_OPEN_METHOD;
	}
	client
		.send_request::<PatchHelperOpen>(PatchHelperOpenParams {
			source_uri: source_uri.to_string(),
			patch_id: patch_id.to_owned(),
			module_content: module_content.to_owned(),
		})
		.await
		.map(drop)
		.map_err(|e| {
			anyhow!("editor does not support vencord/patchHelper/open: {e}")
		})
}

async fn send_close(client: &Client, source_uri: &str, patch_id: &str) {
	struct PatchHelperClose;
	impl Notification for PatchHelperClose {
		type Params = PatchHelperCloseParams;
		const METHOD: &'static str = vencord_ext::PATCH_HELPER_CLOSE_METHOD;
	}
	client
		.send_notification::<PatchHelperClose>(PatchHelperCloseParams {
			source_uri: source_uri.to_owned(),
			patch_id: patch_id.to_owned(),
		})
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
		type Params = PatchHelperUpdateParams;
		const METHOD: &'static str = vencord_ext::PATCH_HELPER_UPDATE_METHOD;
	}
	let reveal_range = reveal.map(|(start_line, end_line)| RevealRange {
		start_line,
		end_line,
	});
	client
		.send_notification::<PatchHelperUpdate>(PatchHelperUpdateParams {
			source_uri: source_uri.to_owned(),
			patch_id: patch_id.to_owned(),
			module_content: module_content.to_owned(),
			reveal_range,
		})
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
	fn relocate_follows_patch_across_replacement_edits() {
		// The user edits the patch's replacements between renders. Same find,
		// same (sole) occurrence -> the helper stays on it regardless of how
		// many replacements it now has.
		const CHANGED: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "stable",
        replacement: { match: /a/, replace: "b" }
    }],
});
"#;
		let relocated =
			relocate_patch(CHANGED, "stable", FindType::String, 0).unwrap();
		assert_eq!(relocated.find_string, "stable");
		assert_eq!(relocated.occurrence, 0);
		assert_eq!(relocated.patch.replacement.len(), 1);
	}

	// Two patches with the same find resolve to the same webpack module; the
	// occurrence index is what keeps them distinct.
	const DUP_FIND: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [
        { find: "shared", replacement: { match: /a/, replace: "first" } },
        { find: "shared", replacement: { match: /b/, replace: "second" } }
    ],
});
"#;

	#[test]
	fn extract_numbers_occurrences_of_a_shared_find() {
		let first = extract_patch(DUP_FIND, 0).unwrap();
		let second = extract_patch(DUP_FIND, 1).unwrap();
		assert_eq!(first.find_string, "shared");
		assert_eq!(second.find_string, "shared");
		// Same find, but distinct occurrences -> distinct identities.
		assert_eq!(first.occurrence, 0);
		assert_eq!(second.occurrence, 1);
		assert_eq!(first.patch.replacement[0].replace.v, Replacer::Str("first".into()));
		assert_eq!(second.patch.replacement[0].replace.v, Replacer::Str("second".into()));
	}

	#[test]
	fn relocate_keeps_each_occurrence_on_its_own_patch() {
		let first =
			relocate_patch(DUP_FIND, "shared", FindType::String, 0).unwrap();
		let second =
			relocate_patch(DUP_FIND, "shared", FindType::String, 1).unwrap();
		assert_eq!(first.occurrence, 0);
		assert_eq!(second.occurrence, 1);
		// The two helpers land on different patches, not the same one.
		assert_eq!(first.patch.replacement[0].replace.v, Replacer::Str("first".into()));
		assert_eq!(second.patch.replacement[0].replace.v, Replacer::Str("second".into()));
	}

	#[test]
	fn relocate_clamps_when_occurrence_disappears() {
		// Started tracking the 2nd "shared" patch, but the source now has only
		// one. Clamp to the survivor instead of tearing the helper down.
		const ONE_LEFT: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [
        { find: "shared", replacement: { match: /a/, replace: "first" } }
    ],
});
"#;
		let relocated =
			relocate_patch(ONE_LEFT, "shared", FindType::String, 1).unwrap();
		assert_eq!(relocated.occurrence, 0);
		assert_eq!(relocated.patch.replacement[0].replace.v, Replacer::Str("first".into()));
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
