use std::cmp::Ordering;

use miette::LabeledSpan;
use oxc::diagnostics::OxcDiagnosticInner;

fn cmp_labaled_span(this: &LabeledSpan, other: &LabeledSpan) -> Ordering {
	this.label()
		.cmp(&other.label())
		.then_with(|| this.inner().cmp(other.inner()))
		.then_with(|| this.primary().cmp(&other.primary()))
}

/// copy of the ordering impl for slices from rust core
fn cmp_slice_labeled_span(
	this: &[LabeledSpan],
	other: &[LabeledSpan],
) -> Ordering {
	let l = this.len().min(other.len());

	let lhs = &this[..l];
	let rhs = &other[..l];

	for i in 0..l {
		match cmp_labaled_span(&lhs[i], &rhs[i]) {
			Ordering::Equal => {}
			ord => return ord,
		}
	}

	this.len().cmp(&other.len())
}

pub fn cmp_oxc_diag(
	this: &OxcDiagnosticInner,
	other: &OxcDiagnosticInner,
) -> Ordering {
	this.message
		.cmp(&other.message)
		.then_with(|| cmp_slice_labeled_span(&this.labels, &other.labels))
		.then_with(|| this.help.cmp(&other.help))
		.then_with(|| this.note.cmp(&other.note))
		.then_with(|| this.severity.cmp(&other.severity))
		.then_with(|| this.code.cmp(&other.code))
		.then_with(|| this.url.cmp(&other.url))
}
