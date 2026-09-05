//! Port of test/data/list-test.ts. The Rust List doesn't share node identity
//! across instances the way the JS one does (which moves a node between
//! lists by mutating its prev/next pointers). Our list stores entries by
//! value, so the "auto-unlink on enqueue elsewhere" behavior is replaced
//! with explicit re-insertion logic in greedy-fas. We test the FIFO
//! semantics that the algorithm actually depends on.

use dagre::{
	graph::NodeIdx,
	list::{FasEntry, List},
};

/// These tests use node ids only as opaque distinct tokens, so map each
/// single-letter name to a fixed index.
fn n(s: &str) -> NodeIdx {
	NodeIdx(u32::from(s.as_bytes()[0]))
}

fn entry(v: &str) -> FasEntry {
	FasEntry {
		v: n(v),
		in_w: 0.0,
		out_w: 0.0,
	}
}

#[test]
fn dequeue_returns_none_on_empty_list() {
	let mut list = List::new();
	assert!(list.dequeue().is_none());
}

#[test]
fn unlinks_and_returns_the_first_entry() {
	let mut list = List::new();
	list.enqueue(entry("a"));
	assert_eq!(list.dequeue().map(|e| e.v), Some(n("a")));
}

#[test]
fn dequeues_in_fifo_order() {
	let mut list = List::new();
	list.enqueue(entry("a"));
	list.enqueue(entry("b"));
	assert_eq!(list.dequeue().map(|e| e.v), Some(n("a")));
	assert_eq!(list.dequeue().map(|e| e.v), Some(n("b")));
}

#[test]
fn remove_by_id() {
	let mut list = List::new();
	list.enqueue(entry("a"));
	list.enqueue(entry("b"));
	let removed = list.remove(n("a"));
	assert_eq!(removed.map(|e| e.v), Some(n("a")));
	assert_eq!(list.dequeue().map(|e| e.v), Some(n("b")));
}
