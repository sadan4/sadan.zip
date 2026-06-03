//! Doubly-linked list used by greedy-fas's bucket queues. Port of
//! `lib/data/list.ts`. We deliberately don't try to share entries by
//! reference like the JS does — instead each bucket owns its entries as a
//! `VecDeque` of records, which is functionally equivalent for the way the
//! algorithm uses it (only enqueue / dequeue from either end, plus moving an
//! entry between buckets by removing & reinserting).
//!
//! In greedy-fas the algorithm "moves" an entry by removing it from one bucket
//! and enqueueing into another. Since the bucket assignment is recomputed
//! from `entry.in / entry.out` each time, we don't need stable references
//! into the list — we just need an efficient queue.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct FasEntry {
	pub v: String,
	pub in_w: f64,
	pub out_w: f64,
}

#[derive(Debug, Default)]
pub struct List {
	items: VecDeque<FasEntry>,
}

impl List {
	pub const fn new() -> Self {
		Self {
			items: VecDeque::new(),
		}
	}

	/// Push to the front. Matches `enqueue` in the JS list, where new entries
	/// are inserted right after the sentinel.
	pub fn enqueue(&mut self, e: FasEntry) {
		self.items.push_front(e);
	}

	/// Pop from the back (sentinel._prev side in JS).
	pub fn dequeue(&mut self) -> Option<FasEntry> {
		self.items.pop_back()
	}

	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	/// Remove the entry with the given `v` if present (used when reassigning
	/// bucket membership after weights change).
	pub fn remove(&mut self, v: &str) -> Option<FasEntry> {
		let pos = self
			.items
			.iter()
			.position(|e| e.v == v)?;
		self.items.remove(pos)
	}
}
