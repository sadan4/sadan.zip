//! Multiline display of an error and its causes, without the backtrace.
//!
//! Implements the API proposed in <https://github.com/dtolnay/anyhow/issues/300>
//! as a free function, since an inherent method can't be added from outside the
//! crate. The output is byte-for-byte what `{:?}` on an `anyhow::Error` prints,
//! minus the trailing backtrace section.
//!
//! ```text
//! failed to load config
//!
//! Caused by:
//!     0: failed to parse `settings.toml`
//!     1: unexpected character at line 4
//! ```
//!
//! Only `std` and `anyhow` are needed; drop this file in as a module.

use std::{
	error::Error as StdError,
	fmt::{self, Display, Write},
};

/// Multiline display: the message, the causes, but no backtrace even if one was
/// captured.
///
/// ```no_run
/// # fn main() -> anyhow::Result<()> {
/// let err = anyhow::anyhow!("root cause").context("outer context");
/// eprintln!("{}", display_no_backtrace(&err));
/// # Ok(())
/// # }
/// ```
pub fn display_no_backtrace(error: &anyhow::Error) -> impl Display + '_ {
	// `anyhow::Error` derefs to `dyn StdError + Send + Sync + 'static`, which
	// coerces to the plain trait object below.
	DisplayNoBacktrace { error: &**error }
}

// /// Same formatting for any `std` error chain, for callers not holding an
// /// `anyhow::Error` (e.g. `display_chain(&io_error)`).
// pub fn display_chain<'a>(error: &'a (dyn StdError + 'static)) -> impl Display + 'a {
//     DisplayNoBacktrace { error }
// }

struct DisplayNoBacktrace<'a> {
	error: &'a (dyn StdError + 'static),
}

impl Display for DisplayNoBacktrace<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.error)?;

		let Some(cause) = self.error.source() else {
			return Ok(());
		};

		f.write_str("\n\nCaused by:")?;

		// A single cause is printed unnumbered, matching anyhow's Debug impl.
		let numbered = cause.source().is_some();
		let mut next = Some(cause);
		let mut n = 0usize;

		while let Some(error) = next {
			f.write_char('\n')?;
			let mut indented = Indented {
				inner: &mut *f,
				number: if numbered { Some(n) } else { None },
				started: false,
			};
			write!(indented, "{error}")?;
			next = error.source();
			n += 1;
		}

		Ok(())
	}
}

/// Writer adapter that indents every line, and prefixes the first line of each
/// cause with its index. Multi-line cause messages stay aligned under their
/// index.
struct Indented<'a, D: ?Sized> {
	inner: &'a mut D,
	number: Option<usize>,
	started: bool,
}

impl<D: ?Sized + Write> Write for Indented<'_, D> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for (i, line) in s.split('\n').enumerate() {
			if !self.started {
				self.started = true;
				match self.number {
					Some(number) => write!(self.inner, "{number: >5}: ")?,
					None => self.inner.write_str("    ")?,
				}
			} else if i > 0 {
				self.inner.write_char('\n')?;
				if self.number.is_some() {
					self.inner.write_str("       ")?;
				} else {
					self.inner.write_str("    ")?;
				}
			}
			self.inner.write_str(line)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::display_no_backtrace;
	use anyhow::anyhow;

	#[test]
	fn no_cause() {
		let err = anyhow!("only me");
		assert_eq!(display_no_backtrace(&err).to_string(), "only me");
	}

	#[test]
	fn single_cause() {
		let err = anyhow!("root").context("outer");
		assert_eq!(
			display_no_backtrace(&err).to_string(),
			"outer\n\nCaused by:\n    root",
		);
	}

	#[test]
	fn multiple_causes_and_multiline_message() {
		let err = anyhow!("root")
			.context("middle\nsecond line")
			.context("outer");
		assert_eq!(
			display_no_backtrace(&err).to_string(),
			"outer\n\nCaused by:\n    0: middle\n       second line\n    1: root",
		);
	}
}
