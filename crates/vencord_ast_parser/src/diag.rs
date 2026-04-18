use std::{borrow::Cow, sync::Arc};

use derive_more::Debug;
use oxc::span::{GetSpan, Span};
use thiserror::Error;

#[derive(Error, Debug, Clone, Default)]
#[error("VencordAstParser: {msg}")]
pub struct ParserDiagnostic {
	pub(crate) msg: Cow<'static, str>,
	pub(crate) labels: Vec<(Span, Cow<'static, str>)>,
	pub(crate) severity: miette::Severity,
	#[debug("({:?})", txt.as_deref().and_then(|s| s.name()).unwrap_or("unknown source"))]
	pub(crate) txt: Option<Arc<dyn miette::SourceCode + Send + Sync + 'static>>,
	pub(crate) cause:
		Option<Arc<dyn miette::Diagnostic + Send + Sync + 'static>>,
}

impl ParserDiagnostic {
	/// attach a source to the error
	pub(crate) fn s(
		mut self,
		cause: impl Into<Box<dyn miette::Diagnostic + Send + Sync + 'static>>,
	) -> Self {
		debug_assert!(self.cause.is_none(), "should only set cause once");
		self.cause = Some(Arc::from(cause.into()));
		self
	}
}

impl miette::Diagnostic for ParserDiagnostic {
	fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		None
	}

	fn severity(&self) -> Option<miette::Severity> {
		Some(self.severity)
	}

	fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		None
	}

	fn note<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		None
	}

	fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		None
	}

	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		self.txt.as_deref().map(|src| src as &_)
	}

	fn labels(
		&self,
	) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
		Some(Box::new(self.labels.iter().map(|(span, label)| {
			if label.is_empty() {
				(*span).into()
			} else {
				miette::LabeledSpan::at(*span, label.clone().into_owned())
			}
		})))
	}

	fn related<'a>(
		&'a self,
	) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
		None
	}

	fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
		self.cause.as_deref().map(|d| d as &_)
	}
}

pub(crate) fn err(
	pos: &impl GetSpan,
	msg: impl Into<Cow<'static, str>>,
) -> ParserDiagnostic {
	ParserDiagnostic {
		msg: msg.into(),
		labels: vec![(pos.span(), "".into())],
		..Default::default()
	}
}

pub(crate) fn err_ns(msg: impl Into<Cow<'static, str>>) -> ParserDiagnostic {
	ParserDiagnostic {
		msg: msg.into(),
		..Default::default()
	}
}

pub type PResult<T> = Result<T, ParserDiagnostic>;

pub(crate) struct NamedSpanContents<'a> {
	inner: Box<dyn miette::SpanContents<'a> + 'a>,
	name: &'a str,
}

impl<'a> miette::SpanContents<'a> for NamedSpanContents<'a> {
	fn data(&self) -> &'a [u8] {
		self.inner.data()
	}

	fn span(&self) -> &miette::SourceSpan {
		self.inner.span()
	}

	fn line(&self) -> usize {
		self.inner.line()
	}

	fn column(&self) -> usize {
		self.inner.column()
	}

	fn line_count(&self) -> usize {
		self.inner.line_count()
	}

	fn name(&self) -> Option<&str> {
		Some(self.name)
	}

	fn language(&self) -> Option<&str> {
		self.inner
			.language()
			.or(Some("JavaScript"))
	}
}

pub struct LocalSource<'a> {
	pub name: &'a str,
	pub source: &'a str,
	pub inner: miette::Report,
}

impl std::fmt::Display for LocalSource<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		<miette::Report as std::fmt::Display>::fmt(&self.inner, f)
	}
}

impl std::fmt::Debug for LocalSource<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let handler = self.inner.handler();
		handler.debug(self, f)
	}
}

impl std::error::Error for LocalSource<'_> {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		self.inner.source()
	}

	fn description(&self) -> &str {
		#[allow(deprecated)]
		self.inner.description()
	}

	fn cause(&self) -> Option<&dyn std::error::Error> {
		#[allow(deprecated)]
		self.inner.cause()
	}
}

impl miette::SourceCode for LocalSource<'_> {
	fn read_span<'a>(
		&'a self,
		span: &miette::SourceSpan,
		context_lines_before: usize,
		context_lines_after: usize,
	) -> std::result::Result<
		Box<dyn miette::SpanContents<'a> + 'a>,
		miette::MietteError,
	> {
		let ret = <str as miette::SourceCode>::read_span(
			self.source,
			span,
			context_lines_before,
			context_lines_after,
		);
		let ret = NamedSpanContents {
			inner: ret?,
			name: self.name,
		};
		Ok(Box::new(ret))
	}

	fn name(&self) -> Option<&str> {
		Some(self.name)
	}
}

impl miette::Diagnostic for LocalSource<'_> {
	fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.inner.code()
	}

	fn severity(&self) -> Option<miette::Severity> {
		self.inner.severity()
	}

	fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.inner.help()
	}

	fn note<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.inner.note()
	}

	fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
		self.inner.url()
	}

	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		Some(self)
	}

	fn labels(
		&self,
	) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
		self.inner.labels()
	}

	fn related<'a>(
		&'a self,
	) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
		self.inner.related()
	}

	fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
		self.inner.diagnostic_source()
	}
}
