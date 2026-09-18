use anyhow::{Context as _, Result};
use ast_parser::diag::{SourceCode, WrappedOxcDiagnostic};
use oxc::{
	allocator::Allocator,
	diagnostics::OxcDiagnostic,
	span::{LabeledSpan, Span},
};
use pretty_printer::{FormattedContent, format_with_alloc, map_pos, map_span};
use smol_str::SmolStr;
use std::sync::Arc;

/// Indent size for rendered formatted sources
const INDENT: u8 = 2;

struct Splice {
	/// The range the replacement covers in the original source.
	original: Span,
	/// The range the replacement covers in the formatted source.
	formatted: Span,
	/// The range the replacement *text* covers in the patched source.
	patched: Span,
	/// The range the replacement *text* covers in the formatted, patched
	/// source.
	both: Span,
}

/// Pretty-prints a syntax error
pub fn format_syntax_error(
	alloc: &Allocator,
	mut e: OxcDiagnostic,
	original_source: &str,
	ranges: &[(Span, String)],
	file_name: Option<&str>,
) -> Result<WrappedOxcDiagnostic> {
	let FormattedContent {
		code: mut formatted_source,
		mappings,
	} = format_with_alloc(original_source, alloc, INDENT)
		.context("Failed to format valid module source")?;
	debug_assert!(ranges.is_sorted_by_key(|(s, _)| s));
	// determine the new ranges and contents of each replacement
	let splices = make_splices(&mappings, ranges);
	// Iterate in reverse (pop()) so that the early ranges dont shift the later ones
	for (splice, (_, repl_txt)) in splices.iter().zip(ranges.iter()).rev() {
		let repl_range = splice.formatted;
		formatted_source.replace_range(
			repl_range.start as usize..repl_range.end as usize,
			repl_txt,
		);
	}

	// Map the error spans from the original diagnostic
	for label in e.labels.as_mut_slice() {
		// miette is evil and doesn't let you mutate the offset or go into string
		let label_span =
			Span::new(label.offset(), label.offset() + label.len());
		// i can't get Option.cloned() to work for some reason
		let txt = label.label().map(String::from);
		let primary = label.primary();
		let new_span = map_label(&splices, &mappings, label_span);
		let new_label = if primary {
			LabeledSpan::new_primary_with_span(txt, new_span)
		} else {
			LabeledSpan::new_with_span(txt, new_span)
		};
		*label = new_label;
	}
	let mut ret = WrappedOxcDiagnostic::from(e);
	ret.source = Some(SourceCode {
		source_code: Arc::from(formatted_source),
		file_name: file_name.map(Into::into),
		file_type: Some(SmolStr::new_static("JavaScript")),
	});
	Ok(ret)
}

fn map_label(patches: &[Splice], mappings: &[(u32, u32)], label: Span) -> Span {
	let start = map_patched_pos(patches, mappings, label.start);
	if label.size() == 0 {
		Span::new(start, start)
	} else {
		// span.end is exclusive, so we need to map the last *inclusive*
		// position and then add 1 to get the new exclusive end
		let end = map_patched_pos(patches, mappings, label.end - 1) + 1;
		Span::new(start, end)
	}
}

/// Maps a position in the patched source to a position in the formatted + patched source
fn map_patched_pos(
	patches: &[Splice],
	mappings: &[(u32, u32)],
	patched_pos: u32,
) -> u32 {
	debug_assert!(patches.is_sorted_by_key(|s| s.patched.start));
	let before = patches.partition_point(|s| s.patched.start <= patched_pos);
	// this position is before every splice, so no changes are needed
	if before == 0 {
		map_pos(mappings, patched_pos)
	} else {
		let idx = before - 1;
		let s = &patches[idx];
		debug_assert!(patched_pos >= s.patched.start);
		// we don't need to map anything if we're within a splice, as spliced text isn't formatted
		if patched_pos < s.patched.end {
			s.both.start + (patched_pos - s.patched.start)
		} else {
			let patched_delta =
				i64::from(s.patched.end) - i64::from(s.original.end);
			let formatted_delta =
				i64::from(s.both.end) - i64::from(s.formatted.end);
			let original_pos = (i64::from(patched_pos) - patched_delta) as u32;
			let formatted_pos = map_pos(mappings, original_pos);
			(i64::from(formatted_pos) + formatted_delta) as u32
		}
	}
}

fn make_splices(
	mappings: &[(u32, u32)],
	replacements: &[(Span, String)],
) -> Vec<Splice> {
	let mut splices = Vec::with_capacity(replacements.len());
	let mut patched_delta: i32 = 0;
	let mut formatted_delta: i32 = 0;
	for (original, repl_txt) in replacements {
		let original = *original;
		let len = repl_txt.len() as u32;
		let formatted = map_span(mappings, original);
		let patched =
			Span::sized((original.start as i32 + patched_delta) as u32, len);
		let both =
			Span::sized((formatted.start as i32 + formatted_delta) as u32, len);
		patched_delta += len as i32 - original.size() as i32;
		formatted_delta += len as i32 - formatted.size() as i32;

		splices.push(Splice {
			original,
			formatted,
			patched,
			both,
		});
	}
	splices
}
