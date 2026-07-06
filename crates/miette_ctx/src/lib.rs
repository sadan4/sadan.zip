use miette::Error;
use std::{convert::Infallible, error::Error as StdErr, fmt::Display};

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
struct DiagnosticError(Box<dyn StdErr + Send + Sync + 'static>);
impl miette::Diagnostic for DiagnosticError {}

pub trait ErrCtx<T, E> {
	/// Wrap the error value with additional context.
	fn context<C>(self, context: C) -> Result<T, Error>
	where
		C: Display + Send + Sync + 'static;

	/// Wrap the error value with additional context that is evaluated lazily
	/// only once an error does occur.
	fn with_context<C, F>(self, context: F) -> Result<T, Error>
	where
		C: Display + Send + Sync + 'static,
		F: FnOnce() -> C;
}

impl<T, E> ErrCtx<T, E> for Result<T, E>
where
	E: Into<Box<dyn StdErr + Send + Sync + 'static>>,
{
	fn context<C>(self, context: C) -> Result<T, Error>
	where
		C: Display + Send + Sync + 'static,
	{
		match self {
			Ok(ok) => Ok(ok),
			Err(err) => {
				Err(Error::from(DiagnosticError(err.into())).context(context))
			}
		}
	}

	fn with_context<C, F>(self, context: F) -> Result<T, Error>
	where
		C: Display + Send + Sync + 'static,
		F: FnOnce() -> C,
	{
		match self {
			Ok(ok) => Ok(ok),
			Err(err) => {
				Err(Error::from(DiagnosticError(err.into())).context(context()))
			}
		}
	}
}

/// ```
/// # type T = ();
/// #
/// use miette_ctx::ErrCtx;
/// use miette::Result;
///
/// fn maybe_get() -> Option<T> {
///     # const IGNORE: &str = stringify! {
///     ...
///     # };
///     # unimplemented!()
/// }
///
/// fn demo() -> Result<()> {
///     let t = maybe_get().context("there is no T")?;
///     # const IGNORE: &str = stringify! {
///     ...
///     # };
///     # unimplemented!()
/// }
/// ```
impl<T> ErrCtx<T, Infallible> for Option<T> {
	fn context<C>(self, context: C) -> Result<T, Error>
	where
		C: Display + Send + Sync + 'static,
	{
		<Self as anyhow::Context<T, Infallible>>::context(self, context)
			.map_err(|e| Error::from(DiagnosticError(e.into())))
	}

	fn with_context<C, F>(self, context: F) -> Result<T, Error>
	where
		C: Display + Send + Sync + 'static,
		F: FnOnce() -> C,
	{
		<Self as anyhow::Context<T, Infallible>>::with_context(self, context)
			.map_err(|e| Error::from(DiagnosticError(e.into())))
	}
}

pub fn map_anyhow(e: anyhow::Error) -> Error {
	Error::from(DiagnosticError(e.into()))
}

pub fn into_anyhow(e: Error) -> anyhow::Error {
	anyhow::Error::from_boxed(e.into())
}
