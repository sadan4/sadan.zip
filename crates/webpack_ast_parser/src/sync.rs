//! A self-owning [`WebpackAstParser`].

use std::{
	mem,
	pin::Pin,
	ptr,
	sync::Arc,
	task::{Context, Poll},
};

use oxc::allocator::Allocator;
use parser_diag::PResult;

use crate::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

/// A [`WebpackAstParser`] that owns both the arena its AST lives in and the
/// source that AST was parsed from.
///
/// Unlike a bare [`WebpackAstParser`], this is not tied to a borrowed arena,
/// so it can be handed out behind an [`Arc`] and kept in a cache.
pub struct ThreadSafeParser {
	/// The parser. The `'static` here is a lie; it really borrows from
	/// [`Self::allocator`] and [`Self::code`].
	inner: WebpackAstParser<'static>,
	/// The arena [`Self::inner`]'s AST is allocated in.
	#[expect(
		dead_code,
		reason = "owns the arena that `inner`'s AST points into"
	)]
	allocator: Box<Allocator>,
	/// The source [`Self::inner`] was parsed from.
	code: Arc<str>,
}

impl ThreadSafeParser {
	/// Parse `code`, taking ownership of it.
	pub fn new(code: Arc<str>) -> PResult<Self> {
		let allocator = Box::new(Allocator::new());
		let inner = WebpackAstParser::try_new(&allocator, &code)?;
		// SAFETY: we hold code and allocator for the lifetime
		// of the parser
		let inner = unsafe {
			mem::transmute::<WebpackAstParser<'_>, WebpackAstParser<'static>>(
				inner,
			)
		};
		Ok(Self {
			inner,
			allocator,
			code,
		})
	}

	/// See [`WebpackAstParser::set_module_cache`].
	///
	/// The cache is the one part of a parser that is real shared state, so it
	/// has to be thread safe for [`Self`]'s `Send`/`Sync` impls to hold.
	pub fn set_module_cache(
		&mut self,
		module_cache: Arc<dyn IModuleCache + Send + Sync>,
	) {
		self.inner
			.set_module_cache(module_cache);
	}

	/// See [`WebpackAstParser::set_module_dep_provider`].
	///
	/// Thread safe for the same reason as [`Self::set_module_cache`].
	pub fn set_module_dep_provider(
		&mut self,
		module_dep_provider: Arc<dyn IModuleDepProvider + Send + Sync>,
	) {
		self.inner
			.set_module_dep_provider(module_dep_provider);
	}

	/// The source this parser was created from.
	#[must_use]
	pub const fn get_source(&self) -> &Arc<str> {
		&self.code
	}

	/// Get access to the inner [`WebpackAstParser`]
	#[must_use]
	pub const fn parser(&self) -> &WebpackAstParser<'_> {
		// SAFETY: `WebpackAstParser<'ast>` is invariant in `'ast` (its caches
		// are `OnceLock`s over types carrying `'ast`), so the shrink cannot be
		// done by coercion. The cast is sound because the allocator and code
		// backing the AST are owned by `self` and live at least as long as
		// this borrow, and the two types have identical layout.
		unsafe { &*ptr::from_ref(&self.inner).cast::<WebpackAstParser<'_>>() }
	}
}

// SAFETY: a parser is read-only once built, and everything it reads is owned by
// the `ThreadSafeParser` that holds it:
//
// - the arena is owned here and is never allocated into again: `try_new` is the
//   only thing that allocates, and a built `WebpackAstParser` keeps no handle
//   on the `Allocator`, so the bump pointer (`Cell<NonNull<u8>>`) is frozen.
// - the AST and semantic data carry `Cell`s (node/scope/symbol ids), but those
//   are written by the parse + semantic build in `try_new` and only read after
//   it, and concurrent reads of a `Cell` race with nothing.
// - the lazy caches are `OnceLock`s, which synchronize their own
//   initialization; what they memoize is derived from the frozen AST.
// - the module cache and dependency provider are the only genuinely shared
//   state, and `ThreadSafeParser::set_module_cache` /
//   `ThreadSafeParser::set_module_dep_provider` only accept `Send + Sync` ones.
//
// Callers must not write through the AST they get from
// `ThreadSafeParser::parser` (e.g. `scope_id.set(..)` on a node); doing so
// breaks the second point above.
unsafe impl Send for ThreadSafeParser {}
// SAFETY: see the `Send` impl above.
unsafe impl Sync for ThreadSafeParser {}

const _: () = {
	const fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<ThreadSafeParser>();
};

/// Asserts that a future is [`Send`] and [`Sync`].
///
/// oxc uses ! [`Send`] and ! [`Sync`] for it's AST; however,
/// the way we use them, they are send + sync
#[repr(transparent)]
pub struct UnsafeFuture<T, O>(T)
where
	T: Future<Output = O>;

impl<T, O> UnsafeFuture<T, O>
where
	T: Future<Output = O>,
{
	/// # Safety
	///
	/// Everything `fut` holds across an await point must be safe to move to,
	/// and share with, another thread
	pub(crate) const unsafe fn new(fut: T) -> Self {
		Self(fut)
	}
}

#[expect(
	clippy::non_send_fields_in_send_ty,
	reason = "asserting `Send` for a future that is not is the whole point"
)]
/// SAFETY: guaranteed by the caller of [`UnsafeFuture::new`].
unsafe impl<T, O> Send for UnsafeFuture<T, O> where T: Future<Output = O> {}
/// SAFETY: see the `Send` impl above.
unsafe impl<T, O> Sync for UnsafeFuture<T, O> where T: Future<Output = O> {}

impl<T, O> Future for UnsafeFuture<T, O>
where
	T: Future<Output = O>,
{
	type Output = O;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		// SAFETY: repr(transparent)
		let fut = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
		fut.poll(cx)
	}
}
