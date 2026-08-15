//! Parser diagnostics for AST parsers
use std::{borrow::Cow, fmt, option::Option, sync::Arc};

use derive_more::Debug;
use miette::SpanContents;
use oxc::span::{GetSpan, Span};
use thiserror::Error;

pub use miette::Severity;

#[derive(Error, Debug, Clone, Default)]
#[error("ParserError: {msg}")]
pub struct ParserDiagnostic {
	pub msg: Cow<'static, str>,
	pub labels: Vec<(Span, Cow<'static, str>)>,
	pub severity: miette::Severity,
	#[debug("({:?})", txt.as_deref().and_then(|s| s.name()).unwrap_or("unknown source"))]
	pub txt: Option<Arc<dyn miette::SourceCode + Send + Sync + 'static>>,
	pub cause: Option<Arc<dyn miette::Diagnostic + Send + Sync + 'static>>,
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

	pub fn with_local_source<'a>(
		self,
		source: &'a str,
		name: &'a str,
	) -> LocalSource<'a> {
		LocalSource {
			name,
			source,
			inner: self.into(),
		}
	}
}

impl miette::Diagnostic for ParserDiagnostic {
	fn code(&self) -> Option<Cow<'_, str>> {
		None
	}

	fn severity(&self) -> Option<miette::Severity> {
		Some(self.severity)
	}

	fn help(&self) -> Option<Cow<'_, str>> {
		None
	}

	fn note(&self) -> Option<Cow<'_, str>> {
		None
	}

	fn url(&self) -> Option<Cow<'_, str>> {
		None
	}

	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		self.txt.as_deref().map(|src| src as &_)
	}

	fn labels(&self) -> miette::Labels {
		self.labels
			.iter()
			.map(|(span, label)| {
				if label.is_empty() {
					(*span).into()
				} else {
					miette::LabeledSpan::at(*span, label.clone().into_owned())
				}
			})
			.collect()
	}

	fn related(&self) -> miette::Related<'_> {
		miette::Related::None
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

// pub(crate) struct NamedSpanContents<'a> {
// 	inner: Box<dyn miette::SpanContents<'a> + 'a>,
// 	name: &'a str,
// }

// impl<'a> miette::SpanContents<'a> for NamedSpanContents<'a> {
// 	fn data(&self) -> &'a [u8] {
// 		self.inner.data()
// 	}

// 	fn span(&self) -> &miette::SourceSpan {
// 		self.inner.span()
// 	}

// 	fn line(&self) -> usize {
// 		self.inner.line()
// 	}

// 	fn column(&self) -> usize {
// 		self.inner.column()
// 	}

// 	fn line_count(&self) -> usize {
// 		self.inner.line_count()
// 	}

// 	fn name(&self) -> Option<&str> {
// 		Some(self.name)
// 	}

// 	fn language(&self) -> Option<&str> {
// 		self.inner
// 			.language()
// 			.or(Some("JavaScript"))
// 	}
// }

pub struct LocalSource<'a> {
	pub name: &'a str,
	pub source: &'a str,
	pub inner: miette::Report,
}

impl fmt::Display for LocalSource<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<miette::Report as fmt::Display>::fmt(&self.inner, f)
	}
}

impl fmt::Debug for LocalSource<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let handler = self.inner.handler();
		handler.debug(self, f)
	}
}

impl std::error::Error for LocalSource<'_> {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		self.inner.source()
	}

	fn description(&self) -> &str {
		#[expect(deprecated)]
		self.inner.description()
	}

	fn cause(&self) -> Option<&dyn std::error::Error> {
		#[expect(deprecated)]
		self.inner.cause()
	}
}

impl miette::SourceCode for LocalSource<'_> {
	fn read_span<'a>(
		&'a self,
		span: &miette::SourceSpan,
		context_lines_before: usize,
		context_lines_after: usize,
	) -> Result<miette::MietteSpanContents<'a>, miette::MietteError> {
		let ret = <str as miette::SourceCode>::read_span(
			self.source,
			span,
			context_lines_before,
			context_lines_after,
		)?;
		let ret = miette::MietteSpanContents::new_named(
			Cow::Borrowed(self.name),
			ret.data(),
			*ret.span(),
			ret.line(),
			ret.column(),
			ret.line_count(),
		);
		Ok(ret)
	}

	fn name(&self) -> Option<&str> {
		Some(self.name)
	}
}

impl miette::Diagnostic for LocalSource<'_> {
	fn code(&self) -> Option<Cow<'_, str>> {
		self.inner.code()
	}

	fn severity(&self) -> Option<miette::Severity> {
		self.inner.severity()
	}

	fn help(&self) -> Option<Cow<'_, str>> {
		self.inner.help()
	}

	fn note(&self) -> Option<Cow<'_, str>> {
		self.inner.note()
	}

	fn url(&self) -> Option<Cow<'_, str>> {
		self.inner.url()
	}

	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		Some(self)
	}

	fn labels(&self) -> miette::Labels {
		self.inner.labels()
	}

	fn related(&self) -> miette::Related<'_> {
		self.inner.related()
	}

	fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
		self.inner.diagnostic_source()
	}
}
