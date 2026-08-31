use std::{
	borrow::Cow,
	fmt::{self, Display},
	option::Option,
	sync::Arc,
};

use derive_more::Debug;
use miette::{LabeledSpan, SourceOffset, SourceSpan, SpanContents};
use oxc::span::{GetSpan, Span};
use thiserror::Error;

pub use miette::Severity;

// unused: clippy bug? used in debug macro func
#[expect(unused)]
fn source_code_name(src: &dyn miette::SourceCode) -> Option<String> {
	src.read_span(&SourceSpan::new(0.into(), 0), 0, 0)
		.ok()?
		.name()
		.map(ToString::to_string)
}

#[derive(Error, Debug, Clone, Default)]
#[error("VencordAstParser: {msg}")]
pub struct ParserDiagnostic {
	pub msg: Cow<'static, str>,
	pub labels: Vec<(Span, Cow<'static, str>)>,
	pub severity: miette::Severity,
	#[debug("({:?})", txt.as_deref().and_then(|s| source_code_name(s)).unwrap_or_else(|| String::from("unknown source")))]
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

fn span_to_source_span(span: Span) -> SourceSpan {
	SourceSpan::new(
		SourceOffset::from(span.start as usize),
		span.size() as usize,
	)
}

impl miette::Diagnostic for ParserDiagnostic {
	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		self.txt.as_deref().map(|src| src as &_)
	}

	fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
		if self.labels.is_empty() {
			None
		} else {
			Some(Box::new(self.labels.iter().map(|(span, label)| {
				let span = span_to_source_span(*span);
				if label.is_empty() {
					LabeledSpan::underline(span)
				} else {
					LabeledSpan::at(span, label.clone().into_owned())
				}
			})))
		}
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
	) -> Result<Box<dyn SpanContents<'a> + 'a>, miette::MietteError> {
		let ret = <str as miette::SourceCode>::read_span(
			self.source,
			span,
			context_lines_before,
			context_lines_after,
		)?;
		let ret = miette::MietteSpanContents::new_named(
			String::from(self.name),
			ret.data(),
			*ret.span(),
			ret.line(),
			ret.column(),
			ret.line_count(),
		);
		Ok(Box::new(ret))
	}
}

impl miette::Diagnostic for LocalSource<'_> {
	fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
		self.inner.code()
	}

	fn severity(&self) -> Option<Severity> {
		self.inner.severity()
	}

	fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
		self.inner.help()
	}

	fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
		self.inner.url()
	}

	fn source_code(&self) -> Option<&dyn miette::SourceCode> {
		Some(self)
	}

	fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
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
