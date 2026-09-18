//! a version of [`oxc::allocator::AllocatorPool`], but gives out mutable references to the allocator
#![deny(missing_docs, clippy::missing_docs_in_private_items)]
use std::{
	iter,
	mem::ManuallyDrop,
	num::NonZeroUsize,
	ops::{Deref, DerefMut},
	sync::Mutex,
	thread,
};

use oxc::allocator::Allocator;
use oxc_data_structures::stack::Stack;

/// A thread-safe pool for reusing [`Allocator`] instances to reduce allocation overhead.
///
/// Uses Standard allocators - suitable for general use.
///
/// ```rs
/// # use ast_parser::pool::AllocPool;
/// # use oxc::allocator::Allocator;
/// let pool: AllocPool;
/// # pool = AllocPool::new(1);
/// {
///     let guard = pool.get();
///     let alloc: &Allocator = &*guard;
/// }
/// {
///     let mut guard = pool.get();
///     let alloc: &mut Allocator = &mut *guard;
/// }
/// ```
pub struct AllocPool {
	/// Allocators currently in the pool.
	/// We use a `Stack` because it's faster than `Vec` for `push` and `pop`,
	/// and those are the operations we do while `Mutex` lock is held.
	/// The shorter the time lock is held, the less contention there is.
	pool: Mutex<Stack<Allocator>>,
}

/// A guard object representing exclusive access to an [`Allocator`] from the pool.
///
/// On drop, the `Allocator` is reset and returned to the pool.
pub struct AllocatorGuard<'pool> {
	/// The allocator that is in use
	alloc: ManuallyDrop<Allocator>,
	/// The pool to return the allocator to
	pool: &'pool AllocPool,
}

impl Deref for AllocatorGuard<'_> {
	type Target = Allocator;

	fn deref(&self) -> &Self::Target {
		&self.alloc
	}
}

impl DerefMut for AllocatorGuard<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.alloc
	}
}

impl Drop for AllocatorGuard<'_> {
	/// Return [`Allocator`] back to the pool.
	fn drop(&mut self) {
		// SAFETY: After taking ownership of the `Allocator`, we do not touch the `ManuallyDrop` again
		let allocator = unsafe { ManuallyDrop::take(&mut self.alloc) };
		self.pool.push(allocator);
	}
}

impl AllocPool {
	/// [`std::thread::available_parallelism()`] or 4 if that fails.
	pub fn default_size() -> usize {
		thread::available_parallelism()
			.ok()
			.map_or(4, NonZeroUsize::get)
	}
	/// Create a new [`AllocPool`] for use across the specified number of threads,
	/// which uses standard allocators.
	///
	/// if `thread_count` is `None`, uses [`Self::default_size()`] to determine the number of allocators to create.
	pub fn new(thread_count: impl Into<Option<usize>>) -> Self {
		let nproc = thread_count
			.into()
			.unwrap_or_else(Self::default_size);
		let pool = iter::repeat_with(Allocator::new)
			.take(nproc)
			.collect();
		Self {
			pool: Mutex::new(pool),
		}
	}

	/// Retrieve an [`Allocator`] from the pool, or create a new one if the pool is empty.
	///
	/// # Panics
	/// Panics if the underlying mutex is poisoned.
	fn pop(&self) -> Allocator {
		let allocator = {
			let mut pool = self.pool.lock().unwrap();
			pool.pop()
		};
		allocator.unwrap_or_else(Allocator::new)
	}

	/// Add an [`Allocator`] to the pool.
	///
	/// The `Allocator` is reset by this method, so it's ready to be re-used.
	///
	/// # Panics
	/// Panics if the underlying mutex is poisoned.
	fn push(&self, mut allocator: Allocator) {
		allocator.reset();
		let mut pool = self.pool.lock().unwrap();
		pool.push(allocator);
	}

	/// Retrieve an [`Allocator`] from the pool, or create a new one if the pool is empty.
	///
	/// Returns an [`AllocatorGuard`] that gives access to the allocator.
	///
	/// # Panics
	///
	/// * Panics if the underlying mutex is poisoned.
	/// * Panics if a new allocator needs to be created but memory allocation fails.
	pub fn get(&self) -> AllocatorGuard<'_> {
		let allocator = self.pop();
		AllocatorGuard {
			alloc: ManuallyDrop::new(allocator),
			pool: self,
		}
	}
}
