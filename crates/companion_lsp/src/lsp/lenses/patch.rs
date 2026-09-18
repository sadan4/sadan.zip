//! Code lenses on the patches of a vencord plugin.

use std::{ffi::OsStr, path::Path};

use parser_diag::LocalSource;
use serde::{Deserialize, Serialize};
use tower_lsp_server::ls_types::{CodeLens, Command, Uri};
use tracing::{debug, trace};
use vencord_ast_parser::{Allocator, VencordAstParser};

use crate::lsp::{self, doc::Document};

const LENSES: &[(&str, &str)] = &[
	("View Module", "extract_search"),
	("Diff Module", "diff_module_search"),
	("Test Patch", "test_patch"),
	("Open in Patch Helper", "open_patch_helper"),
];

mod hash_repr;

/// Serialized patch lens args
#[derive(Serialize, Deserialize)]
pub struct PatchLensArgs {
	pub uri: Uri,
	#[serde(with = "hash_repr")]
	pub hash: u64,
}

/// does `path` match the glob
/// `**/*plugins{,/_*}/{*.ts,*.tsx,**/index.ts,**/index.tsx}`
fn is_plugin_path(path: &Path) -> bool {
	if !matches!(path.extension().and_then(OsStr::to_str), Some("ts" | "tsx")) {
		return false;
	}
	let is_index = path
		.file_stem()
		.is_some_and(|stem| stem == "index");
	let parts: Vec<&str> = path
		.components()
		.filter_map(|c| c.as_os_str().to_str())
		.collect();
	parts
		.iter()
		.enumerate()
		// the last part is the file itself, so a `plugins` dir there has
		// nothing under it
		.rev()
		.skip(1)
		.filter(|(_, part)| part.ends_with("plugins"))
		.any(|(i, _)| match &parts[i + 1..] {
			// `*plugins/*.ts`
			[_] => true,
			// `*plugins/_*/*.ts`
			[dir, _] if dir.starts_with('_') => true,
			// `*plugins/**/index.ts`, with or without the `_*` segment
			_ => is_index,
		})
}

pub(super) fn patch_lenses(
	uri: &Uri,
	doc: &Document,
	path: &Path,
) -> Vec<CodeLens> {
	if !is_plugin_path(path) {
		trace!(?path, "not a plugin file, skipping patch lenses");
		return Vec::new();
	}
	let src = doc.text.as_str();
	let path_str = path.to_string_lossy();
	let alloc = Allocator::new();
	let parser = match VencordAstParser::try_new(&alloc, src, Some(&path_str)) {
		Ok(parser) => parser,
		Err(e) => {
			let e = LocalSource {
				name: &path_str,
				source: src,
				inner: e,
			};
			debug!("Failed to parse plugin file, skipping patch lenses:{e:?}");
			return Vec::new();
		}
	};
	let patches = match parser.patches(true) {
		Ok(patches) => patches,
		Err(e) => {
			let e = LocalSource {
				name: &path_str,
				source: src,
				inner: miette::Report::from(e),
			};
			debug!("Failed to parse patches, skipping patch lenses:{e:?}");
			return Vec::new();
		}
	};
	let mut ret = Vec::with_capacity(patches.len() * LENSES.len());
	for patch in patches {
		let range = doc.range_for_span(patch.span);
		let hash = patch.content_hash();
		let args = match serde_json::to_value(PatchLensArgs {
			uri: uri.clone(),
			hash,
		}) {
			Ok(args) => args,
			Err(e) => {
				debug!("Failed to serialize lens arguments, skipping: {e}");
				continue;
			}
		};
		ret.extend(
			LENSES
				.iter()
				.map(|&(title, cmd)| CodeLens {
					range,
					command: Some(Command {
						title: String::from(title),
						command: lsp::Server::cmd_name(cmd),
						arguments: Some(vec![args.clone()]),
					}),
					data: None,
				}),
		);
	}
	ret
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_paths() {
		const CASES: &[(&str, bool)] = &[
			("/a/src/plugins/foo.ts", true),
			("/a/src/plugins/foo.tsx", true),
			("/a/src/plugins/index.ts", true),
			("/a/src/plugins/foo/index.tsx", true),
			("/a/src/plugins/foo/bar/index.ts", true),
			("/a/src/userplugins/foo.ts", true),
			("/a/src/equicordplugins/foo/index.ts", true),
			("/a/src/equicordplugins/_api/thing.ts", true),
			("/a/src/plugins/_core/settings.tsx", true),
			("/a/src/plugins/_core/deep/index.ts", true),
			("/a/src/plugins/foo/bar.ts", false),
			("/a/src/plugins/_core/deep/other.ts", false),
			("/a/src/plugin/foo.ts", false),
			("/a/src/plugins/foo.js", false),
			("/a/src/plugins", false),
			("/a/plugins.ts", false),
		];
		for &(path, want) in CASES {
			assert_eq!(
				is_plugin_path(Path::new(path)),
				want,
				"is_plugin_path({path:?})"
			);
		}
	}
}
