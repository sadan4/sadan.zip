use std::cmp::Ordering;

pub type CodePoint = u32;

/// An inclusive range of code points.
/// This is more efficient than `InclusiveRange` because it does not need to carry
/// around the `Option<bool>`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Interval {
	pub first: CodePoint,
	pub last: CodePoint,
}

/// A list of sorted, inclusive, non-empty ranges of code points.
impl Interval {
	pub const fn new(first: CodePoint, last: CodePoint) -> Self {
		debug_assert!(first <= last);
		Self { first, last }
	}

	#[inline(always)]
	pub const fn compare(self, cp: u32) -> Ordering {
		if self.first > cp {
			Ordering::Greater
		} else if self.last < cp {
			Ordering::Less
		} else {
			Ordering::Equal
		}
	}
}

pub fn interval_contains(interval: &[Interval], cp: u32) -> bool {
	interval
		.binary_search_by(|iv| iv.compare(cp))
		.is_ok()
}
