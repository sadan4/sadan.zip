use deno_tower_lsp::{
	jsonrpc::Result as LspResult,
	lsp_types::{CodeLens, CodeLensParams, Command, Position, Range, Uri},
};
use oxc::{allocator::Allocator, span::Span};
use serde::Serialize;
use serde_json::Value;
use vencord_ast_parser::{FindArg, FindUse, VencordAstParser};

use crate::{
	discord_bridge::messages::{
		DisablePluginData,
		FindData,
		FindNode,
		RegexValue,
	},
	lsp::{self, Backend, get_doc},
	vencord_ext,
};

pub async fn code_lens(
	backend: &Backend,
	params: CodeLensParams,
) -> LspResult<Option<Vec<CodeLens>>> {
	let Some(doc) = get_doc(&backend.state, &params.text_document.uri) else {
		return Ok(None);
	};

	// Parsing is CPU-bound — push it off the executor.
	let lenses = tokio::task::spawn_blocking(move || {
		patch_code_lenses(&doc.text, &doc.uri)
	})
	.await
	.ok()
	.flatten();

	Ok(lenses)
}

/// Emit:
/// * "Test Patch" + "Open in Patch Helper" per patch produced by
///   `VencordAstParser`. The `patchIndex` argument is the patch's index
///   in `parser.patches()` — the same index `cmd_test_patch` uses, so
///   click-time resolution is a plain `Vec::get(index)`.
/// * "Enable Plugin" + "Disable Plugin" anchored on the `definePlugin`
///   object literal, dispatching to `vencord.disablePlugin` with the
///   appropriate `enabled` flag. We can't read the live plugin state
///   from disk, so we emit both buttons and let the user pick.
fn patch_code_lenses(source: &str, uri: &Uri) -> Option<Vec<CodeLens>> {
	let alloc = Allocator::default();
	let parser = VencordAstParser::try_new(&alloc, source, None).ok()?;
	// LSP never runs patches through `regress` — it only uses the patches'
	// spans and ships them to Discord — so skip regress-only canonicalization.
	// Errors here typically mean "no definePlugin in this file", which is
	// fine for the find lenses below.
	let patches = parser
		.patches(false)
		.unwrap_or_else(|e| {
			tracing::debug!(?e, "patches() failed; skipping patch lenses");
			Vec::new()
		});
	// `extract_patches` returns one slot per parser patch — `None` for patches
	// that can't be expressed on the wire (e.g. function replacements). We
	// only emit lenses for `Some` slots so users don't click a button that
	// errors out, but we use the slot's index as `patchIndex` so it lines up
	// with what `cmd_test_patch` looks up.
	let wire = lsp::diagnostics::extract_patches(source);
	let plugin = parser.plugin_info();

	let uri_str = uri.to_string();
	let mut out: Vec<CodeLens> = Vec::with_capacity(
		patches.len() * 2 + if plugin.is_ok() { 2 } else { 0 },
	);
	for (index, patch) in patches.iter().enumerate() {
		if !matches!(wire.get(index), Some(Some(_))) {
			continue;
		}
		let range = span_to_range(source, patch.span);
		out.push(make_patch_lens(
			range,
			"Test Patch",
			vencord_ext::CMD_TEST_PATCH,
			&uri_str,
			index,
		));
		out.push(make_patch_lens(
			range,
			"Open in Patch Helper",
			vencord_ext::CMD_OPEN_PATCH_HELPER,
			&uri_str,
			index,
		));
	}
	match plugin {
		Ok(plugin) => {
			let range = span_to_range(source, plugin.span);
			out.push(make_plugin_lens(
				range,
				"Enable Plugin",
				&plugin.name,
				true,
			));
			out.push(make_plugin_lens(
				range,
				"Disable Plugin",
				&plugin.name,
				false,
			));
		}
		Err(e) => {
			tracing::debug!(e =? e.with_local_source(source, &uri_str), "plugin_info() failed; skipping plugin lenses");
		}
	}

	// Webpack `find*()` calls — independent of `definePlugin`. Errors here
	// just mean "no @webpack import" or "unsupported arg shape"; drop them
	// rather than throwing away the patch/plugin lenses we already have.
	match parser.get_finds() {
		Ok(finds) => {
			for find in finds {
				let range = span_to_range(source, find.range);
				out.push(make_find_lens(range, "View Module", true, find));
				out.push(make_find_lens(range, "Test Find", false, find));
			}
		}
		Err(e) => {
			tracing::debug!(e =? e.clone().with_local_source(source, &uri_str), "get_finds() failed; skipping find lenses");
		}
	}
	Some(out)
}

fn span_to_range(source: &str, span: Span) -> Range {
	let ((start_line, start_col), (end_line, end_col)) =
		ast_parser::span_line_and_column(source, span);
	Range {
		start: Position {
			line: start_line,
			character: start_col,
		},
		end: Position {
			line: end_line,
			character: end_col,
		},
	}
}

/// Serialize a code-lens command argument into the `Value` that LSP's
/// `Command.arguments` array holds. These are plain data structs, so the
/// conversion can't fail in practice.
fn to_arg<T: Serialize>(value: T) -> Value {
	serde_json::to_value(value).expect("code-lens argument serializes to JSON")
}

/// `{ uri, patchIndex }` — the shape `cmd_test_patch` / `patch_helper::open`
/// deserialize back when the lens is clicked.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchLensArg<'a> {
	uri: &'a str,
	patch_index: usize,
}

/// Legacy `vencord.extractFind` payload: `{ extractType: "find", findType,
/// findArgs }`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractFindArgs {
	extract_type: &'static str,
	find_type: String,
	find_args: Vec<FindNode>,
}

fn make_patch_lens(
	range: Range,
	title: &str,
	command_id: &str,
	uri: &str,
	patch_index: usize,
) -> CodeLens {
	CodeLens {
		range,
		command: Some(Command {
			title: title.to_owned(),
			command: command_id.to_owned(),
			arguments: Some(vec![to_arg(PatchLensArg { uri, patch_index })]),
		}),
		data: None,
	}
}

fn find_arg_to_node(arg: &FindArg) -> FindNode {
	match arg {
		FindArg::String(s) => FindNode::String { value: s.clone() },
		FindArg::Regex { pattern, flags } => FindNode::Regex {
			value: RegexValue {
				pattern: pattern.clone(),
				flags: flags.clone(),
			},
		},
		FindArg::Function(s) => FindNode::Function { value: s.clone() },
	}
}

/// `extract` flips the lens into "View Module" mode (calls
/// `vencord.extractFind` with the legacy `{ extractType: "find", findType,
/// findArgs }` payload). `extract = false` produces the "Test Find" lens,
/// which calls `vencord.testFind` with the wire `FindData` shape
/// (`{ type, args }`) that `cmd_test_find` deserializes directly.
fn make_find_lens(
	range: Range,
	title: &str,
	extract: bool,
	find: &FindUse,
) -> CodeLens {
	let kind = find.data.kind.to_string();
	let args: Vec<FindNode> = find
		.data
		.args
		.iter()
		.map(find_arg_to_node)
		.collect();
	let (command_id, payload): (&str, Value) = if extract {
		(
			vencord_ext::CMD_EXTRACT_FIND,
			to_arg(ExtractFindArgs {
				extract_type: "find",
				find_type: kind,
				find_args: args,
			}),
		)
	} else {
		// `{ type, args }` — the wire `FindData` shape `cmd_test_find` parses.
		(vencord_ext::CMD_TEST_FIND, to_arg(FindData { kind, args }))
	};
	CodeLens {
		range,
		command: Some(Command {
			title: title.to_owned(),
			command: command_id.to_owned(),
			arguments: Some(vec![payload]),
		}),
		data: None,
	}
}

fn make_plugin_lens(
	range: Range,
	title: &str,
	plugin_name: &str,
	enabled: bool,
) -> CodeLens {
	CodeLens {
		range,
		command: Some(Command {
			title: title.to_owned(),
			command: vencord_ext::CMD_DISABLE_PLUGIN.to_owned(),
			// The wire `DisablePluginData` shape `cmd_disable_plugin` parses.
			arguments: Some(vec![to_arg(DisablePluginData {
				enabled,
				plugin_name: plugin_name.to_owned(),
			})]),
		}),
		data: None,
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr as _;

	use super::*;

	const PLUGIN_SOURCE: &str = r#"import definePlugin from "@utils/types";

export default definePlugin({
    name: "MyPlugin",
    patches: [
        {
            find: "foo.bar",
            replacement: { match: /a/, replace: "b" }
        },
        {
            find: /baz/,
            replacement: [{ match: "x", replace: "y" }]
        },
    ],
});
"#;

	#[test]
	fn emits_two_lenses_per_patch_plus_plugin_pair() {
		let uri = Uri::from_str("file:///test/MyPlugin/index.ts").unwrap();
		let lenses = patch_code_lenses(PLUGIN_SOURCE, &uri).unwrap();
		assert_eq!(lenses.len(), 6, "expected 2 patches x 2 + plugin x 2");
		let titles: Vec<_> = lenses
			.iter()
			.map(|l| {
				l.command
					.as_ref()
					.unwrap()
					.title
					.as_str()
			})
			.collect();
		assert_eq!(
			titles,
			vec![
				"Test Patch",
				"Open in Patch Helper",
				"Test Patch",
				"Open in Patch Helper",
				"Enable Plugin",
				"Disable Plugin",
			]
		);
	}

	#[test]
	fn plugin_lens_carries_name_and_enabled_flag() {
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let lenses = patch_code_lenses(PLUGIN_SOURCE, &uri).unwrap();
		let enable = lenses
			.iter()
			.find(|l| l.command.as_ref().unwrap().title == "Enable Plugin")
			.expect("enable lens missing");
		let cmd = enable.command.as_ref().unwrap();
		assert_eq!(cmd.command, vencord_ext::CMD_DISABLE_PLUGIN);
		let arg = &cmd.arguments.as_ref().unwrap()[0];
		assert_eq!(arg["pluginName"], "MyPlugin");
		assert_eq!(arg["enabled"], true);

		let disable = lenses
			.iter()
			.find(|l| l.command.as_ref().unwrap().title == "Disable Plugin")
			.expect("disable lens missing");
		assert_eq!(
			disable
				.command
				.as_ref()
				.unwrap()
				.arguments
				.as_ref()
				.unwrap()[0]["enabled"],
			false,
		);
	}

	#[test]
	fn lens_command_carries_uri_and_patch_index() {
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let lenses = patch_code_lenses(PLUGIN_SOURCE, &uri).unwrap();
		let first_cmd = lenses[0].command.as_ref().unwrap();
		assert_eq!(first_cmd.command, vencord_ext::CMD_TEST_PATCH);
		let arg = &first_cmd.arguments.as_ref().unwrap()[0];
		assert_eq!(arg["uri"], "file:///x.ts");
		assert_eq!(arg["patchIndex"], 0);
	}

	#[test]
	fn lens_range_covers_whole_patch_object() {
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let lenses = patch_code_lenses(PLUGIN_SOURCE, &uri).unwrap();
		// Both lenses for the same patch share the patch object's range.
		assert_eq!(lenses[0].range, lenses[1].range);
		// The range should span multiple lines (the patch literal is multi-line).
		assert!(lenses[0].range.end.line > lenses[0].range.start.line);
	}

	#[test]
	fn function_replacement_still_gets_lenses() {
		// Regression for "patch index 1 out of range": the second patch uses
		// an arrow-function `replace`. We ship the function's JS source as a
		// `function` wire node (Discord-side `eval`s it), so the lens should
		// be emitted and its `patchIndex` should match the parser's order.
		let src = r##"import definePlugin from "@utils/types";
export default definePlugin({
    name: "P",
    patches: [
        {
            find: ".ToastType.FORWARD",
            replacement: [{ match: /a/, replace: "b" }]
        },
        {
            find: "#{intl::MESSAGE_FORWARD_MESSAGE_PLACEHOLDER}",
            replacement: {
                match: /(useCallback\(\()(\)=>\{)(\i\.\i\.clearDraft)/,
                replace: (_, a, b, c) => `${a}arg${b}${c}`
            }
        }
    ],
});
"##;
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let lenses = patch_code_lenses(src, &uri).unwrap_or_default();
		let indices: Vec<_> = lenses
			.iter()
			.filter(|l| l.command.as_ref().unwrap().title == "Test Patch")
			.map(|l| {
				l.command
					.as_ref()
					.unwrap()
					.arguments
					.as_ref()
					.unwrap()[0]["patchIndex"]
					.as_u64()
					.unwrap()
			})
			.collect();
		assert_eq!(
			indices,
			vec![0, 1],
			"both patches should get a Test Patch lens with indices matching \
			 the parser's source order"
		);
	}

	#[test]
	fn no_lenses_when_no_define_plugin() {
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let src = "export const x = 1;";
		let lenses = patch_code_lenses(src, &uri).unwrap_or_default();
		assert_eq!(lenses, []);
	}

	#[test]
	fn emits_view_and_test_lenses_per_find_call() {
		let src = r#"import { findByProps, findComponentLazy } from "@webpack";
const a = findByProps("foo", "bar");
const b = findComponentLazy(/baz/i);
"#;
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let lenses = patch_code_lenses(src, &uri).unwrap_or_default();
		let titles: Vec<_> = lenses
			.iter()
			.map(|l| {
				l.command
					.as_ref()
					.unwrap()
					.title
					.as_str()
			})
			.collect();
		assert_eq!(
			titles,
			vec!["View Module", "Test Find", "View Module", "Test Find"]
		);

		let view = &lenses[0].command.as_ref().unwrap();
		assert_eq!(view.command, vencord_ext::CMD_EXTRACT_FIND);
		let view_arg = &view.arguments.as_ref().unwrap()[0];
		assert_eq!(view_arg["extractType"], "find");
		assert_eq!(view_arg["findType"], "findByProps");
		assert_eq!(view_arg["findArgs"][0]["type"], "string");
		assert_eq!(view_arg["findArgs"][0]["value"], "foo");

		let test = &lenses[1].command.as_ref().unwrap();
		assert_eq!(test.command, vencord_ext::CMD_TEST_FIND);
		let test_arg = &test.arguments.as_ref().unwrap()[0];
		assert_eq!(test_arg["type"], "findByProps");
		assert_eq!(test_arg["args"][0]["type"], "string");
		assert_eq!(test_arg["args"][0]["value"], "foo");

		// Second call: regex arg + Lazy suffix.
		let test_b = &lenses[3].command.as_ref().unwrap();
		let test_b_arg = &test_b.arguments.as_ref().unwrap()[0];
		assert_eq!(test_b_arg["type"], "findComponentLazy");
		assert_eq!(test_b_arg["args"][0]["type"], "regex");
		assert_eq!(test_b_arg["args"][0]["value"]["pattern"], "baz");
		assert_eq!(test_b_arg["args"][0]["value"]["flags"], "i");
	}

	#[test]
	fn no_find_lenses_when_no_webpack_import() {
		let src = "const a = findByProps(\"foo\");";
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let lenses = patch_code_lenses(src, &uri).unwrap_or_default();
		assert_eq!(lenses, []);
	}

	#[test]
	fn plugin_lenses_emit_even_without_patches() {
		// A plugin with no patches should still get the Enable/Disable pair —
		// they don't depend on patch metadata.
		let uri = Uri::from_str("file:///x.ts").unwrap();
		let src = r#"import definePlugin from "@utils/types";
export default definePlugin({ name: "P" });
"#;
		let lenses = patch_code_lenses(src, &uri).unwrap_or_default();
		let titles: Vec<_> = lenses
			.iter()
			.map(|l| {
				l.command
					.as_ref()
					.unwrap()
					.title
					.as_str()
			})
			.collect();
		assert_eq!(titles, vec!["Enable Plugin", "Disable Plugin"]);
	}
}
