use std::{fmt::Write as _, sync::LazyLock};

use oxc::allocator::Allocator;
use regex::Regex;
use tower_lsp::{
	jsonrpc::Result as LspResult,
	lsp_types::{
		Hover,
		HoverContents,
		HoverParams,
		MarkupContent,
		MarkupKind,
		Position,
		Range,
	},
};
use vencord_ast_parser::hash::hash_message_key;
use webpack_ast_parser::WebpackAstParser;

use crate::{
	lsp::{Backend, get_doc},
	state::Document,
};

/// `#{intl::SOME_KEY}` and `#{intl::SOME_KEY::raw}` patterns inside string
/// literals in Vencord plugin sources. Matches independently of language.
static VENCORD_INTL_RE: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"#\{intl::([\w$+/]+)(?:::(\w+))?\}").unwrap());

/// Result of locating an i18n token under the cursor.
struct IntlToken {
	/// 6-char hashed key suitable for `i18n_lookup`.
	hashed: String,
	/// Source key when we have one (Vencord pattern); `None` for the webpack
	/// path where the identifier IS the hash.
	source: Option<String>,
	range: Range,
}

pub async fn hover(
	backend: &Backend,
	params: HoverParams,
) -> LspResult<Option<Hover>> {
	let uri = params
		.text_document_position_params
		.text_document
		.uri
		.clone();
	let pos = params
		.text_document_position_params
		.position;
	let Some(doc) = get_doc(&backend.state, &uri) else {
		return Ok(None);
	};

	let Some(token) = locate_intl_token(&doc, pos) else {
		// TODO(phase 5): WebpackAstParser::generate_hover for export type
		// info — requires the .modules/ cache + IModuleCache impl to resolve
		// re-exports across modules.
		return Ok(None);
	};

	let value = match backend
		.state
		.i18n_cache
		.get(&token.hashed)
	{
		Some(v) => Some(v.value().clone()),
		None if backend
			.state
			.discord
			.is_connected()
			.await =>
		{
			match backend
				.state
				.discord
				.i18n_lookup(&token.hashed)
				.await
			{
				Ok(v) => {
					backend
						.state
						.i18n_cache
						.insert(token.hashed.clone(), v.clone());
					Some(v)
				}
				Err(e) => {
					tracing::debug!(?e, "i18n lookup failed");
					None
				}
			}
		}
		None => None,
	};

	Ok(Some(make_hover(&token, value.as_deref())))
}

fn locate_intl_token(doc: &Document, pos: Position) -> Option<IntlToken> {
	// 1. Vencord-side `#{intl::KEY}` — pure text regex, works in any language.
	if let Some(t) = locate_vencord_intl(&doc.text, pos) {
		return Some(t);
	}

	// 2. Webpack-side i.t.HASH — AST-driven; only meaningful in extracted
	//    `.js` module files (matches the legacy provider's language filter).
	if doc.language_id == "javascript"
		&& let Some(t) = locate_webpack_intl(&doc.text, pos)
	{
		return Some(t);
	}

	None
}

fn locate_vencord_intl(text: &str, pos: Position) -> Option<IntlToken> {
	let offset = ast_parser::get_offset_from_line_and_column(
		text,
		pos.line,
		pos.character,
	) as usize;

	let m = VENCORD_INTL_RE
		.find_iter(text)
		.find(|m| range_contains(m.start(), m.end(), offset))?;
	let caps = VENCORD_INTL_RE.captures_at(text, m.start())?;
	let source_key = caps.get(1)?.as_str();
	let modifier = caps.get(2).map(|c| c.as_str());

	let hashed = if modifier == Some("raw") {
		source_key.to_owned()
	} else {
		hash_message_key(source_key)
			.iter()
			.collect::<String>()
	};

	Some(IntlToken {
		hashed,
		source: Some(source_key.to_owned()),
		range: byte_range_to_lsp(text, m.start(), m.end()),
	})
}

/// AST-driven webpack i18n hash detection. Uses
/// `WebpackAstParser::get_i18n_key_at` which only matches when the cursor is
/// on a 6-char identifier or string literal whose parent is a member
/// expression — semantically equivalent to `i.t.HASH` / `i.t["HASH"]` without
/// the regex false-positives (e.g. a 6-char variable name elsewhere).
fn locate_webpack_intl(text: &str, pos: Position) -> Option<IntlToken> {
	let offset = ast_parser::get_offset_from_line_and_column(
		text,
		pos.line,
		pos.character,
	);

	let alloc = Allocator::default();
	let parser = WebpackAstParser::try_new(&alloc, text).ok()?;
	let (span, key) = parser.get_i18n_key_at(offset)?;

	Some(IntlToken {
		hashed: key.to_string(),
		source: None,
		range: byte_range_to_lsp(text, span.start as usize, span.end as usize),
	})
}

const fn range_contains(start: usize, end: usize, point: usize) -> bool {
	point >= start && point <= end
}

fn byte_range_to_lsp(text: &str, start: usize, end: usize) -> Range {
	let (sl, sc) = ast_parser::get_line_and_column(text, start as u32);
	let (el, ec) = ast_parser::get_line_and_column(text, end as u32);
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

fn make_hover(token: &IntlToken, value: Option<&str>) -> Hover {
	let mut md = String::new();
	if let Some(src) = &token.source {
		_ = writeln!(md, "**Intl key:** `{src}`  ");
	}
	_ = writeln!(md, "**Hashed:** `{}`  ", token.hashed);
	match value {
		Some(v) => {
			_ = write!(md, "\n```\n{v}\n```");
		},
		None => md.push_str(
			"\n*Connect Discord with `vc-userDevTools` to resolve the localized string.*",
		),
	}
	Hover {
		contents: HoverContents::Markup(MarkupContent {
			kind: MarkupKind::Markdown,
			value: md,
		}),
		range: Some(token.range),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tower_lsp::lsp_types::Url;

	fn doc(language_id: &str, text: &str) -> Document {
		Document {
			uri: Url::parse("file:///t").unwrap(),
			version: 1,
			language_id: language_id.into(),
			text: text.into(),
		}
	}

	#[test]
	fn locates_vencord_intl_pattern_and_hashes_key() {
		let d = doc("typescript", r##"const x = "#{intl::APP_TAG}";"##);
		let pos = Position {
			line: 0,
			character: 22,
		};
		let t = locate_intl_token(&d, pos).unwrap();
		assert_eq!(t.source.as_deref(), Some("APP_TAG"));
		assert_eq!(t.hashed, "9RNkeF");
	}

	#[test]
	fn raw_modifier_skips_hashing() {
		let d = doc("typescript", r##""#{intl::abcDEF::raw}""##);
		let pos = Position {
			line: 0,
			character: 12,
		};
		let t = locate_intl_token(&d, pos).unwrap();
		assert_eq!(t.source.as_deref(), Some("abcDEF"));
		assert_eq!(t.hashed, "abcDEF");
	}

	#[test]
	fn webpack_intl_lookup_uses_ast_for_member_access() {
		// `i.t.AbCdEf` — identifier as member of `t`.
		let d = doc("javascript", "function _(i){return i.t.AbCdEf}");
		let pos = Position {
			line: 0,
			character: 27,
		}; // inside AbCdEf
		let t = locate_intl_token(&d, pos).unwrap();
		assert_eq!(t.hashed, "AbCdEf");
		assert!(t.source.is_none());
	}

	#[test]
	fn webpack_intl_lookup_handles_bracket_form() {
		let d = doc("javascript", r#"function _(i){return i.t["XyZ012"]}"#);
		let pos = Position {
			line: 0,
			character: 28,
		};
		let t = locate_intl_token(&d, pos).unwrap();
		assert_eq!(t.hashed, "XyZ012");
	}

	#[test]
	fn locates_webpack_intl_in_patch_helper_module() {
		// Patch Helper renders the patched module with a `// Webpack Module N`
		// header and a `0,` prefix (so the anonymous module function parses as
		// an expression). Hover must resolve `i.t.HASH` inside that content.
		let src =
			"// Webpack Module 123\n0,\nfunction(e,t,i){return i.t.AbCdEf}";
		let d = doc("javascript", src);
		// Cursor inside `AbCdEf` on the third line.
		let pos = Position {
			line: 2,
			character: 30,
		};
		let t = locate_intl_token(&d, pos).unwrap();
		assert_eq!(t.hashed, "AbCdEf");
		assert!(t.source.is_none());
	}

	#[test]
	fn ast_rejects_non_member_six_char_identifiers() {
		// `AbCdEf` here is a top-level binding, not a member access — the
		// old regex would have matched, the AST path correctly does not.
		let d = doc("javascript", "var AbCdEf=1;");
		let pos = Position {
			line: 0,
			character: 7,
		};
		assert!(locate_intl_token(&d, pos).is_none());
	}

	#[test]
	fn webpack_path_skipped_in_non_javascript_documents() {
		// Same source as the previous regex-style test but flagged as TS:
		// without the language gate this would still match `i.t.AbCdEf`,
		// but the AST path is JS-only.
		let d = doc("typescript", "function _(i){return i.t.AbCdEf}");
		let pos = Position {
			line: 0,
			character: 27,
		};
		assert!(locate_intl_token(&d, pos).is_none());
	}

	#[test]
	fn returns_none_outside_any_pattern() {
		let d = doc("typescript", r#"const x = "hello";"#);
		let pos = Position {
			line: 0,
			character: 12,
		};
		assert!(locate_intl_token(&d, pos).is_none());
	}
}
