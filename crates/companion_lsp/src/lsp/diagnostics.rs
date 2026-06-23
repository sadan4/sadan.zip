use std::{sync::mpsc, time::Duration};

use oxc::{ast::ast::RegExpFlags, span::Span};
use tower_lsp::lsp_types::{
	Diagnostic,
	DiagnosticSeverity,
	Position,
	Range,
	Url,
};
use vencord_ast_parser::{
	VencordAstParser,
	diag::{ParserDiagnostic, Severity},
};

use crate::{
	discord_bridge::messages::{
		FindType,
		MatchNode,
		PatchData,
		RegexValue,
		ReplaceNode,
		Replacement,
	},
	state::Document,
};

/// Window between the user typing and re-running validation. Matches the
/// legacy `vencord-user-companion.diagnosticRefreshTimeout` default of 1.5s.
pub const DEBOUNCE: Duration = Duration::from_millis(1_500);

/// Public entry point invoked from `document::on_did_open` and
/// `document::on_did_change`. Spawns a debounced refresh that aborts itself
/// if a newer document version arrives or no Discord client is connected.
pub fn schedule(
	backend_state: crate::state::SharedState,
	client: tower_lsp::Client,
	uri: Url,
) {
	tokio::spawn(async move {
		let Some(start_version) = backend_state
			.documents
			.get(&uri)
			.map(|d| d.version)
		else {
			return;
		};

		tokio::time::sleep(DEBOUNCE).await;

		// Bail if a newer edit has arrived during the debounce window.
		let current = backend_state
			.documents
			.get(&uri)
			.map(|d| d.clone());
		let Some(doc) = current else { return };
		if doc.version != start_version {
			return;
		}

		// Parser-side lints (unused capture groups, bad backrefs, etc.) don't
		// need Discord — surface them either way. When Discord is connected we
		// also test each patch against the live bundle and merge the results.
		match run_validation(&doc, &backend_state).await {
			Ok(diags) => {
				client
					.publish_diagnostics(uri.clone(), diags, Some(doc.version))
					.await;
			}
			Err(e) => {
				tracing::debug!(?e, %uri, "diagnostics run failed");
			}
		}
	});
}

async fn run_validation(
	doc: &Document,
	state: &crate::state::SharedState,
) -> anyhow::Result<Vec<Diagnostic>> {
	// Parsing is CPU-bound — push it to a blocking thread, then re-acquire
	// owned data before returning to the async context.
	let text_for_patches = doc.text.clone();
	let text_for_lints = doc.text.clone();
	let source = doc.text.clone();

	let parsed =
		tokio::task::spawn_blocking(move || extract_patches(&text_for_patches))
			.await
			.unwrap_or(Vec::new());

	let parser_lints = tokio::task::spawn_blocking(move || {
		collect_parser_diagnostics(&text_for_lints)
	})
	.await
	.unwrap_or_default();

	let mut diags = parser_lints;

	if state.discord.is_connected().await {
		for (span, wire) in parsed.into_iter().flatten() {
			if let Err(e) = state.discord.test_patch(wire).await {
				let range = span_to_range(&source, span);
				diags.push(Diagnostic {
					range,
					severity: Some(DiagnosticSeverity::ERROR),
					source: Some("vencord".into()),
					message: e.to_string(),
					..Default::default()
				});
			}
		}
	}
	Ok(diags)
}

/// Runs `VencordAstParser::collect_diagnostics` on the source and converts the
/// resulting `ParserDiagnostic`s into LSP `Diagnostic`s. Diagnostics without a
/// primary label span are dropped — they can't be anchored in the editor.
pub(crate) fn collect_parser_diagnostics(source: &str) -> Vec<Diagnostic> {
	use oxc::allocator::Allocator;

	let alloc = Allocator::default();
	let Ok(mut parser) = VencordAstParser::try_new(&alloc, source, None) else {
		return Vec::new();
	};
	let (tx, rx) = mpsc::channel();
	parser.diag_ch = Some(tx);
	parser.collect_diagnostics();
	// `collect_diagnostics` is synchronous — every send has already happened
	// by the time it returns, so `try_iter` drains without blocking.
	rx.try_iter()
		.filter_map(|d| parser_diagnostic_to_lsp(source, &d))
		.collect()
}

fn parser_diagnostic_to_lsp(
	source: &str,
	diag: &ParserDiagnostic,
) -> Option<Diagnostic> {
	let (primary_span, _) = diag.labels.first()?;
	let range = span_to_range(source, *primary_span);

	let mut message = diag.msg.clone().into_owned();
	for (_, label) in diag.labels.iter().skip(1) {
		if label.is_empty() {
			continue;
		}
		message.push_str("\n  • ");
		message.push_str(label);
	}

	Some(Diagnostic {
		range,
		severity: Some(severity_to_lsp(diag.severity)),
		source: Some("vencord".into()),
		message,
		..Default::default()
	})
}

const fn severity_to_lsp(sev: Severity) -> DiagnosticSeverity {
	match sev {
		Severity::Error => DiagnosticSeverity::ERROR,
		Severity::Warning => DiagnosticSeverity::WARNING,
		Severity::Advice => DiagnosticSeverity::INFORMATION,
	}
}

/// Walks the source via `VencordAstParser`, converting each parser `Patch`
/// into a wire-format `PatchData` paired with the patch's source `Span` for
/// diagnostic placement. Patches that can't be expressed on the wire (e.g.
/// non-string/regex finds or matches) come back as `None` so the returned
/// `Vec`'s indices line up 1:1 with `VencordAstParser::patches()` — callers
/// (code lenses, the `testPatch` resolver) rely on that alignment.
///
/// Function replacements ship as `ReplaceNode::Function` carrying the raw JS
/// source of the arrow function, which the Discord-side handler `eval`s back
/// into a callable — same protocol as the original TS extension.
pub(crate) fn extract_patches(source: &str) -> Vec<Option<(Span, PatchData)>> {
	use oxc::allocator::Allocator;
	use vencord_ast_parser::Replacer;

	let alloc = Allocator::default();
	let Ok(parser) = VencordAstParser::try_new(&alloc, source, None) else {
		return Vec::new();
	};
	// Pass `false`: the wire payload goes to Discord's JS runtime, which
	// uses JS regex / replacement semantics — applying regress-specific
	// rewrites here would corrupt the patch.
	let Ok(patches) = parser.patches(false) else {
		return Vec::new();
	};

	let mut out = Vec::with_capacity(patches.len());
	for patch in patches {
		let Some((find_type, find)) = match_to_wire(&patch.find.v) else {
			out.push(None);
			continue;
		};

		let mut replacements = Vec::with_capacity(patch.replacement.len());
		let mut skip_patch = false;
		for r in &patch.replacement {
			let Some(m) = match_to_wire_node(&r.match_.v) else {
				skip_patch = true;
				break;
			};
			let replace = match &r.replace.v {
				Replacer::Str(s) => ReplaceNode::String { value: s.clone() },
				// The parser keeps the original arrow function's span on the
				// `ReplaceLike` wrapper — slice the source and ship that
				// verbatim as a `function` node.
				Replacer::Template(_) => {
					let span = r.replace.s;
					let Some(text) =
						source.get(span.start as usize..span.end as usize)
					else {
						skip_patch = true;
						break;
					};
					ReplaceNode::Function {
						value: text.to_owned(),
					}
				}
			};
			replacements.push(Replacement { match_: m, replace });
		}
		if skip_patch {
			out.push(None);
			continue;
		}

		let data = PatchData {
			find_type,
			find,
			replacement: replacements,
		};
		out.push(Some((patch.find.s, data)));
	}

	out
}

fn match_to_wire(m: &vencord_ast_parser::Match) -> Option<(FindType, String)> {
	use vencord_ast_parser::Match;
	match m {
		Match::Str(finder) => {
			let s = str::from_utf8(finder.needle()).ok()?;
			Some((FindType::String, s.to_owned()))
		}
		Match::Regex(r) => {
			Some((FindType::Regex, stringify_regex(&r.pattern, r.flags)))
		}
	}
}

fn match_to_wire_node(m: &vencord_ast_parser::Match) -> Option<MatchNode> {
	use vencord_ast_parser::Match;
	match m {
		Match::Str(finder) => {
			let s = str::from_utf8(finder.needle()).ok()?;
			Some(MatchNode::String {
				value: s.to_owned(),
			})
		}
		Match::Regex(r) => Some(MatchNode::Regex {
			value: RegexValue {
				pattern: r.pattern.clone(),
				flags: flags_to_string(r.flags),
			},
		}),
	}
}

fn stringify_regex(pattern: &str, flags: RegExpFlags) -> String {
	format!("/{pattern}/{}", flags_to_string(flags))
}

fn flags_to_string(flags: RegExpFlags) -> String {
	let mut s = String::new();
	if flags.contains(RegExpFlags::G) {
		s.push('g');
	}
	if flags.contains(RegExpFlags::I) {
		s.push('i');
	}
	if flags.contains(RegExpFlags::M) {
		s.push('m');
	}
	if flags.contains(RegExpFlags::S) {
		s.push('s');
	}
	if flags.contains(RegExpFlags::U) {
		s.push('u');
	}
	if flags.contains(RegExpFlags::Y) {
		s.push('y');
	}
	s
}

fn span_to_range(source: &str, span: Span) -> Range {
	let ((sl, sc), (el, ec)) = ast_parser::span_line_and_column(source, span);
	Range {
		start: Position {
			line: sl,
			character: sc,
		},
		end: Position {
			line: el,
			character: ec,
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const PLUGIN: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [
        {
            find: "foo.bar",
            replacement: { match: "a", replace: "b" }
        },
        {
            find: /baz/g,
            replacement: [{ match: /x/i, replace: "y" }]
        }
    ],
});
"#;

	#[test]
	fn extracts_string_patch_to_wire() {
		let patches: Vec<_> = extract_patches(PLUGIN)
			.into_iter()
			.flatten()
			.collect();
		assert!(!patches.is_empty(), "expected at least one wire patch");
		let (_, data) = &patches[0];
		match data.find_type {
			FindType::String => {}
			FindType::Regex => panic!("expected string find"),
		}
		assert_eq!(data.find, "foo.bar");
		assert_eq!(data.replacement.len(), 1);
		match &data.replacement[0].match_ {
			MatchNode::String { value } => assert_eq!(value, "a"),
			MatchNode::Regex { .. } => panic!("expected string match"),
		}
		match &data.replacement[0].replace {
			ReplaceNode::String { value } => assert_eq!(value, "b"),
			ReplaceNode::Function { .. } => panic!("expected string replace"),
		}
	}

	#[test]
	fn extracts_regex_patch_to_wire_with_flags() {
		let patches: Vec<_> = extract_patches(PLUGIN)
			.into_iter()
			.flatten()
			.collect();
		let regex_patch = patches
			.iter()
			.find(|(_, d)| matches!(d.find_type, FindType::Regex));
		let (_, data) = regex_patch.expect("expected a regex patch");
		assert_eq!(data.find, "/baz/g");
		match &data.replacement[0].match_ {
			MatchNode::Regex { value } => {
				assert_eq!(value.pattern, "x");
				assert_eq!(value.flags, "i");
			}
			MatchNode::String { .. } => panic!("expected regex match"),
		}
	}

	#[test]
	fn wire_replacement_preserves_js_capture_syntax() {
		// `$&` and `$<name>` are JS replacement syntax. The parser rewrites
		// them to `$0` / `${name}` internally so `regress` can evaluate, but
		// the wire payload must keep the JS form — Discord runs
		// `String.prototype.replace` semantics. Regression for "Replacement
		// failed: SyntaxError: Unexpected token ')'".
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "abc",
        replacement: { match: /(?<name>foo)/, replace: "before $& after $<name>!" }
    }],
});
"#;
		let patches: Vec<_> = extract_patches(SRC)
			.into_iter()
			.flatten()
			.collect();
		let (_, data) = patches
			.first()
			.expect("expected wire patch");
		match &data.replacement[0].replace {
			ReplaceNode::String { value } => {
				assert_eq!(value, "before $& after $<name>!");
				assert!(!value.contains("$0"), "should not ship regress form");
				assert!(
					!value.contains("${name}"),
					"should not ship regress form"
				);
			}
			ReplaceNode::Function { .. } => panic!("expected string replace"),
		}
	}

	#[test]
	fn function_replacement_ships_as_function_node() {
		// Discord-side `eval`s the `function`-node value back into a callable,
		// so we ship the arrow function's raw JS source verbatim.
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "abc",
        replacement: {
            match: /(useCallback\(\()(\)=>\{)/,
            replace: (_, a, b) => `${a}arg${b}`
        }
    }],
});
"#;
		let patches: Vec<_> = extract_patches(SRC)
			.into_iter()
			.flatten()
			.collect();
		let (_, data) = patches
			.first()
			.expect("expected wire patch");
		match &data.replacement[0].replace {
			ReplaceNode::Function { value } => {
				assert!(
					value.starts_with("(_, a, b)"),
					"expected raw arrow source, got {value:?}"
				);
				assert!(
					value.contains("`${a}arg${b}`"),
					"expected template body preserved, got {value:?}"
				);
			}
			ReplaceNode::String { .. } => panic!("expected function replace"),
		}
	}

	#[test]
	fn flag_serialization_matches_js_order() {
		let mut f = RegExpFlags::empty();
		f.insert(RegExpFlags::I);
		f.insert(RegExpFlags::G);
		assert_eq!(flags_to_string(f), "gi");
	}

	#[test]
	fn collects_parser_warning_for_global_flag_on_find() {
		// `g` on a find regex is a no-op; the parser flags it as a warning.
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: /foo/g,
        replacement: { match: /x/, replace: "y" }
    }],
});
"#;
		let diags = collect_parser_diagnostics(SRC);
		assert!(
			diags
				.iter()
				.any(|d| d.severity == Some(DiagnosticSeverity::WARNING)
					&& d.message.contains("global flag")),
			"expected global-flag warning, got {diags:#?}"
		);
	}

	#[test]
	fn collects_parser_error_for_nonexistent_capture_group() {
		const SRC: &str = r#"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [{
        find: "abc",
        replacement: { match: /(a)/, replace: "$2" }
    }],
});
"#;
		let diags = collect_parser_diagnostics(SRC);
		assert!(
			diags
				.iter()
				.any(|d| d.severity == Some(DiagnosticSeverity::ERROR)
					&& d.message
						.contains("non-existent capture group")),
			"expected non-existent-capture-group error, got {diags:#?}"
		);
	}
}
