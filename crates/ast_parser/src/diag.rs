//! Wrappers to help interface [`oxc`] with [`miette`];

use std::{
	borrow::{Borrow, Cow},
	cmp::Ordering,
	fmt::{self, Debug, Display},
	ops::{Index, IndexMut},
	sync::Arc,
};

use derive_more::From;
use miette::{Diagnostic, LabeledSpan, Severity};
use oxc::{
	diagnostics::{OxcCode, OxcDiagnostic, OxcDiagnosticInner},
	span::Span,
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smol_str::SmolStr;

type Labels = SmallVec<[LabeledSpan; 2]>;
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct OwnOxcCode {
	pub scope: Option<Cow<'static, str>>,
	pub number: Option<Cow<'static, str>>,
}

impl Display for OwnOxcCode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match (&self.scope, &self.number) {
			(Some(scope), Some(number)) => write!(f, "{scope}({number})"),
			(Some(scope), None) => Display::fmt(scope, f),
			(None, Some(number)) => Display::fmt(number, f),
			(None, None) => Ok(()),
		}
	}
}

impl From<OxcCode> for OwnOxcCode {
	fn from(o: OxcCode) -> Self {
		Self {
			scope: o.scope,
			number: o.number,
		}
	}
}

#[derive(
	Debug,
	Serialize,
	Deserialize,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
)]
pub struct OxcSourceSpan {
	pub start: u32,
	pub end: u32,
}

impl IndexMut<OxcSourceSpan> for str {
	fn index_mut(&mut self, index: OxcSourceSpan) -> &mut Self::Output {
		&mut self[index.start as usize..index.end as usize]
	}
}

impl Index<OxcSourceSpan> for str {
	type Output = Self;

	fn index(&self, index: OxcSourceSpan) -> &Self::Output {
		&self[index.start as usize..index.end as usize]
	}
}

impl OxcSourceSpan {
	#[must_use]
	pub const fn len(&self) -> u32 {
		self.end - self.start
	}

	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl From<Span> for OxcSourceSpan {
	fn from(value: Span) -> Self {
		Self {
			start: value.start,
			end: value.end,
		}
	}
}

impl From<OxcSourceSpan> for miette::SourceSpan {
	fn from(value: OxcSourceSpan) -> Self {
		let size = (value.end - value.start) as usize;
		Self::new((value.start as usize).into(), size)
	}
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnOxcDiagnostic {
	pub message: Cow<'static, str>,
	pub labels: Labels,
	pub help: Option<Cow<'static, str>>,
	pub note: Option<Cow<'static, str>>,
	pub severity: Severity,
	pub code: OwnOxcCode,
	pub url: Option<Cow<'static, str>>,
}

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

impl PartialOrd for OwnOxcDiagnostic {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for OwnOxcDiagnostic {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		let Self {
			message,
			labels,
			help,
			note,
			severity,
			code,
			url,
		} = self;
		match message.cmp(&other.message) {
			Ordering::Equal => {}
			ord => return ord,
		}
		match cmp_slice_labeled_span(labels, &other.labels) {
			Ordering::Equal => {}
			ord => return ord,
		}
		match help.cmp(&other.help) {
			Ordering::Equal => {}
			ord => return ord,
		}
		match note.cmp(&other.note) {
			Ordering::Equal => {}
			ord => return ord,
		}
		match severity.cmp(&other.severity) {
			Ordering::Equal => {}
			ord => return ord,
		}
		match code.cmp(&other.code) {
			Ordering::Equal => {}
			ord => return ord,
		}
		url.cmp(&other.url)
	}
}

impl From<OxcDiagnosticInner> for OwnOxcDiagnostic {
	fn from(o: OxcDiagnosticInner) -> Self {
		Self {
			message: o.message,
			labels: o
				.labels
				.into_iter()
				.map(|label| {
					if label.primary() {
						LabeledSpan::new_primary_with_span(
							label.label().map(ToString::to_string),
							miette::SourceSpan::new(
								(label.offset() as usize).into(),
								label.len() as usize,
							),
						)
					} else {
						LabeledSpan::new(
							label.label().map(ToString::to_string),
							label.offset() as usize,
							label.len() as usize,
						)
					}
				})
				.collect(),
			help: o.help,
			note: o.note,
			severity: match o.severity {
				oxc::diagnostics::Severity::Advice => Severity::Advice,
				oxc::diagnostics::Severity::Warning => Severity::Warning,
				oxc::diagnostics::Severity::Error => Severity::Error,
			},
			code: o.code.into(),
			url: o.url,
		}
	}
}

#[derive(
	derive_more::Debug,
	Serialize,
	Deserialize,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Clone,
)]
pub struct SourceCode {
	#[debug(skip)]
	pub source_code: Arc<str>,
	pub file_name: Option<SmolStr>,
	pub file_type: Option<SmolStr>,
}

impl miette::SourceCode for SourceCode {
	fn read_span<'a>(
		&'a self,
		span: &miette::SourceSpan,
		context_lines_before: usize,
		context_lines_after: usize,
	) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
		struct W<'a> {
			file_name: Option<&'a str>,
			file_type: Option<&'a str>,
			contents: Box<dyn miette::SpanContents<'a> + 'a>,
		}
		impl<'a> miette::SpanContents<'a> for W<'a> {
			fn data(&self) -> &'a [u8] {
				self.contents.data()
			}

			fn span(&self) -> &miette::SourceSpan {
				self.contents.span()
			}

			fn line(&self) -> usize {
				self.contents.line()
			}

			fn column(&self) -> usize {
				self.contents.column()
			}

			fn line_count(&self) -> usize {
				self.contents.line_count()
			}

			fn name(&self) -> Option<&str> {
				self.file_name
			}

			fn language(&self) -> Option<&str> {
				self.file_type
			}
		}
		let span_contents = self.source_code.read_span(
			span,
			context_lines_before,
			context_lines_after,
		)?;
		Ok(Box::new(W {
			file_name: self.file_name.as_deref(),
			file_type: self.file_type.as_deref(),
			contents: span_contents,
		}))
	}
}

#[derive(
	From, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
pub struct WrappedOxcDiagnostic {
	pub inner: OwnOxcDiagnostic,
	pub source: Option<SourceCode>,
}

impl Borrow<dyn Diagnostic> for Box<WrappedOxcDiagnostic> {
	fn borrow(&self) -> &(dyn Diagnostic + 'static) {
		&**self
	}
}

impl WrappedOxcDiagnostic {
	#[must_use]
	pub fn with_source_code(mut self, source: impl Into<Arc<str>>) -> Self {
		self.source = Some(SourceCode {
			source_code: source.into(),
			file_name: None,
			file_type: None,
		});
		self
	}
	const fn new(diag: OwnOxcDiagnostic) -> Self {
		Self {
			inner: diag,
			source: None,
		}
	}
}

impl From<OxcDiagnostic> for WrappedOxcDiagnostic {
	fn from(diag: OxcDiagnostic) -> Self {
		Self::new(diag.inner_owned().into())
	}
}

impl From<OxcDiagnosticInner> for WrappedOxcDiagnostic {
	fn from(diag: OxcDiagnosticInner) -> Self {
		Self::new(diag.into())
	}
}

impl Display for WrappedOxcDiagnostic {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		Display::fmt(&self.inner.message, f)
	}
}

impl std::error::Error for WrappedOxcDiagnostic {}

impl miette::Diagnostic for WrappedOxcDiagnostic {
	fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
		if self.inner.code.scope.is_none() && self.inner.code.number.is_none() {
			None
		} else {
			Some(Box::new(&self.inner.code))
		}
	}

	fn severity(&self) -> Option<miette::Severity> {
		Some(self.inner.severity)
	}

	fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
		match self.inner.help.as_deref() {
			Some(help) => Some(Box::new(help)),
			None => None,
		}
	}

	fn url<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
		match self.inner.url.as_deref() {
			Some(url) => Some(Box::new(url)),
			None => None,
		}
	}

	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		self.source.as_ref().map(|s| s as _)
	}

	fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
		if self.inner.labels.is_empty() {
			None
		} else {
			Some(Box::new(self.inner.labels.iter().cloned()))
		}
	}
}
