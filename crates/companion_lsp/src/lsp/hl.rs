use std::sync::Arc;

use oxc::span::Span;
use tokio::task;
use tower_lsp::{
	jsonrpc::Result as LspResult,
	lsp_types::{self, DocumentHighlight, DocumentHighlightParams},
};
use vencord_ast_parser::{Match, MatchRegex, Patch, Replacement};

use crate::{Backend, lsp};

pub async fn document_highlight(
	backend: &Backend,
	params: DocumentHighlightParams,
) -> LspResult<Option<Vec<DocumentHighlight>>> {
	let Some(doc) = lsp::get_doc(
		&backend.state,
		&params
			.text_document_position_params
			.text_document
			.uri,
	) else {
		return Ok(None);
	};

	let state = Arc::clone(&backend.state);
	let highlights = task::spawn_blocking(move || {
		let pos = params
			.text_document_position_params
			.position;
		let offset = ast_parser::get_offset_from_line_and_column(
			&doc.text,
			pos.line,
			pos.character,
		);
		// Reuses the cached parse for `doc`'s version when one exists, so
		// repeated highlight requests on an unchanged document don't re-parse.
		let patches = lsp::get_patches(&state, &doc)?;
		highlights_from_patches(&patches, &doc.text, offset)
	})
	.await;
	match highlights {
		Ok(hl) => Ok(hl),
		Err(e) => {
			tracing::debug!(?e, "document_highlight task panicked");
			Ok(None)
		}
	}
}

fn find_relevant_patch(patches: &[Patch], pos: u32) -> Option<&Patch> {
	let pred = |patch: &&Patch| pos >= patch.span.start && pos < patch.span.end;
	if cfg!(debug_assertions) {
		let mut found = patches.iter().filter(pred);
		let first = found.next();
		assert!(
			found.next().is_none(),
			"Multiple patches found at position {pos}"
		);
		first
	} else {
		patches.iter().find(pred)
	}
}

/// `Vec<Map<GroupIdx, Vec<Span>>>`
fn flatten_patch_capture_spans(
	patch: &Patch,
) -> impl Iterator<Item = Vec<Vec<Span>>> + '_ {
	patch
		.replacement
		.iter()
		.map(flatten_replacement_capture_spans)
}

fn flatten_replacement_capture_spans(
	replacement: &Replacement,
) -> Vec<Vec<Span>> {
	// `Span` is `Copy`, so this clones cheaply; we only do it for the single
	// relevant patch, not the whole document.
	let mut spans = replacement
		.replace
		.used_replace_capture_spans
		.clone();
	match &replacement.match_.v {
		Match::Regex(MatchRegex { capture_spans, .. }) => {
			for (i, span) in capture_spans.iter().enumerate() {
				let g_idx = i + 1;
				if g_idx >= spans.len() {
					// the span is not used anywhere in the replacement
					// we can ignore it. it will get highlighted as a warning anyway.
					// we will need to handle this case if we ever highlight backreferences
					continue;
				}
				spans[g_idx].push(*span);
			}
		}
		Match::Str(_) => {
			// we are not handling highlighting the whole match as of now;
		}
	}
	spans
}

/// given a bunch of spans, returns the smallest size of the span that includes pos
///
/// if no span includes pos, returns None
fn smallest_covering_span(pos: u32, spans: &[Span]) -> Option<u32> {
	let mut ret = None;
	for span in spans {
		if pos >= span.start && pos < span.end {
			let size = span.size();
			if let Some(cur) = ret {
				if size < cur {
					ret = Some(size);
				}
			} else {
				ret = Some(size);
			}
		}
	}
	ret
}

fn highlights_from_patches(
	patches: &[Patch],
	src: &str,
	pos: u32,
) -> Option<Vec<DocumentHighlight>> {
	let patch = find_relevant_patch(patches, pos)?;

	// take the regex /(foo(bar))/
	// if our cursor is at   a|r
	// then we will be within both capture groups 1 and 2
	// in order to disambiguate which one to highlight
	// we take the one with the smallest size, as that must be the innermost
	let (_, spans) = flatten_patch_capture_spans(patch)
		.flatten()
		.filter_map(|spans| {
			smallest_covering_span(pos, &spans).map(|size| (size, spans))
		})
		.min_by(|a, b| a.0.cmp(&b.0))?;

	let ranges = spans
		.into_iter()
		.map(|span| {
			let (sl, sc) = ast_parser::get_line_and_column(src, span.start);
			let (el, ec) = ast_parser::get_line_and_column(src, span.end);
			let range = lsp_types::Range {
				start: lsp_types::Position {
					line: sl,
					character: sc,
				},
				end: lsp_types::Position {
					line: el,
					character: ec,
				},
			};
			DocumentHighlight { range, kind: None }
		})
		.collect();
	Some(ranges)
}

#[cfg(test)]
mod tests {
	use insta::assert_ron_snapshot;
	use oxc::allocator::Allocator;
	use vencord_ast_parser::VencordAstParser;

	use super::*;

	#[test]
	fn works_for_every_pos_in_first_span() {
		let src = include_str!(
			"../../../vencord_ast_parser/src/tests/data/plugin10.tsx"
		);
		let alloc = Allocator::new();
		let ast = VencordAstParser::try_new(&alloc, src, None).unwrap();
		let patches = ast.patches(false).unwrap();
		let span = patches[0].replacement[0]
			.match_
			.v
			.unwrap_regex_ref()
			.capture_spans[1];
		let mut foo = Vec::with_capacity(span.size() as _);
		for i in span.start..span.end {
			let res = highlights_from_patches(&patches, src, i).unwrap();
			foo.push(res);
		}
		// would use a hash set, but DocumentHighlight doesn't impl hash
		foo.dedup();
		assert_eq!(foo.len(), 1);
		assert_ron_snapshot!(foo.swap_remove(0), @"
		[
		  DocumentHighlight(
		    range: Range(
		      start: Position(
		        line: 44,
		        character: 48,
		      ),
		      end: Position(
		        line: 44,
		        character: 50,
		      ),
		    ),
		  ),
		  DocumentHighlight(
		    range: Range(
		      start: Position(
		        line: 42,
		        character: 42,
		      ),
		      end: Position(
		        line: 42,
		        character: 54,
		      ),
		    ),
		  ),
		]
		");
	}
}
