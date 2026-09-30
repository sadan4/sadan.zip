use std::{
	borrow::Cow,
	fmt::{self, Display, Write},
	option::Option,
	sync::Arc,
};

use derive_more::Debug;
use miette::{LabeledSpan, SourceOffset, SourceSpan, SpanContents};
use oxc_span::{GetSpan, Span};
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
#[error("ParserDiagnostic: {msg}")]
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
	#[must_use]
	pub fn s(
		mut self,
		cause: impl Into<Box<dyn miette::Diagnostic + Send + Sync + 'static>>,
	) -> Self {
		debug_assert!(self.cause.is_none(), "should only set cause once");
		self.cause = Some(Arc::from(cause.into()));
		self
	}

	/// Rewrite every label offset with `f`.
	///
	/// Used to move a diagnostic from the coordinates of the source it was
	/// parsed from into the coordinates of the text it is rendered against,
	/// eg the pretty printed module.
	///
	/// Only this diagnostic's own labels are remapped; a [`Self::cause`]
	/// attached with [`Self::s`] is an opaque [`miette::Diagnostic`] and keeps
	/// its original offsets.
	#[must_use]
	pub fn remap_spans(mut self, f: impl Fn(u32) -> u32) -> Self {
		for (span, _) in &mut self.labels {
			*span = Span::new(f(span.start), f(span.end));
		}
		self
	}

	#[must_use]
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

pub fn err(
	pos: &impl GetSpan,
	msg: impl Into<Cow<'static, str>>,
) -> ParserDiagnostic {
	ParserDiagnostic {
		msg: msg.into(),
		labels: vec![(pos.span(), "".into())],
		..Default::default()
	}
}

pub fn slice_span<T: GetSpan>(slice: &[T]) -> Option<Span> {
	if slice.is_empty() {
		return None;
	}
	Some(Span::new(
		slice.first().unwrap().span().start,
		slice.last().unwrap().span().end,
	))
}

pub fn err_ns(msg: impl Into<Cow<'static, str>>) -> ParserDiagnostic {
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
		f.write_char('\n')?;
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

/// Lines longer than this are cut down to [`LONG_LINE_PADDING`] around the
/// span when rendering, see [`LocalSource::long_line_window`]
const MAX_LINE_LEN: usize = 512;
/// How much of a long line is shown on each side of the span
const LONG_LINE_PADDING: usize = 80;
/// The most of a long line that is shown, even if the span is longer
const MAX_WINDOW_LEN: usize = 1024;

impl LocalSource<'_> {
	/// If the lines `span` is on are too long to render, the part of them
	/// around `span` and its location in [`Self::source`]
	///
	/// Minified code is often one line, which miette would render in full. It
	/// also pads underlines with `{:width$}`, which panics once a span starts
	/// more than [`u16::MAX`] columns into a line.
	fn long_line_window(
		&self,
		span: &SourceSpan,
	) -> Option<(&[u8], SourceSpan)> {
		let src = self.source;
		let start = span.offset().min(src.len());
		let end = (start + span.len()).min(src.len());
		let line_start = src[..start]
			.rfind('\n')
			.map_or(0, |i| i + 1);
		let line_end = src[end..]
			.find('\n')
			.map_or(src.len(), |i| end + i);
		if line_end - line_start <= MAX_LINE_LEN {
			return None;
		}
		let win_start = src.floor_char_boundary(
			start
				.saturating_sub(LONG_LINE_PADDING)
				.max(line_start),
		);
		// a span that crosses a newline keeps the newline, so the window ends
		// on the line the span ends on
		let win_end = src.ceil_char_boundary(
			(end + LONG_LINE_PADDING)
				.min(line_end)
				.min(win_start + MAX_WINDOW_LEN),
		);
		Some((
			&src.as_bytes()[win_start..win_end],
			SourceSpan::new(win_start.into(), win_end - win_start),
		))
	}

	/// Whether `span` is a label of this diagnostic or one of its causes
	fn is_label(&self, span: &SourceSpan) -> bool {
		let mut diag: Option<&dyn miette::Diagnostic> =
			Some(self.inner.as_ref());
		while let Some(d) = diag {
			if d.labels()
				.is_some_and(|mut labels| labels.any(|l| l.inner() == span))
			{
				return true;
			}
			diag = d.diagnostic_source();
		}
		false
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
		let (data, data_span, line_count) = match self.long_line_window(span) {
			// miette reads the span covering two labels to try to render them
			// as one snippet, only merging them if that read succeeds. Refuse
			// when that span doesn't fit in a window, so both labels are shown.
			Some((_, data_span))
				if data_span.len() < span.len() && !self.is_label(span) =>
			{
				return Err(miette::MietteError::OutOfBounds);
			}
			Some((data, data_span)) => (data, data_span, 1),
			None => (ret.data(), *ret.span(), ret.line_count()),
		};
		let ret = miette::MietteSpanContents::new_named(
			String::from(self.name),
			data,
			data_span,
			ret.line(),
			ret.column(),
			line_count,
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

#[cfg(test)]
mod tests;
