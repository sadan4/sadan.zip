use std::cmp::{self, Ordering};

pub type CodePoint = u32;

/// The maximum (inclusive) code point.
pub const CODE_POINT_MAX: CodePoint = 0x10FFFF;

/// An inclusive range of code points.
/// This is more efficient than InclusiveRange because it does not need to carry
/// around the `Option<bool>`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Interval {
    pub first: CodePoint,
    pub last: CodePoint,
}

/// A list of sorted, inclusive, non-empty ranges of code points.
impl Interval {
    pub const fn new(first: CodePoint, last: CodePoint) -> Interval {
        debug_assert!(first <= last);
        Interval { first, last }
    }


    #[inline(always)]
    pub fn compare(self, cp: u32) -> Ordering {
        if self.first > cp {
            Ordering::Greater
        } else if self.last < cp {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }


    /// Return whether self is before rhs.
    fn is_before(self, other: Interval) -> bool {
        self.last < other.first
    }


    /// Return whether self is strictly before rhs.
    /// "Strictly" here means there is at least one value after the end of self,
    /// and before the start of rhs. Overlapping *or abutting* intervals are
    /// not considered strictly before.
    fn is_strictly_before(self, rhs: Interval) -> bool {
        self.last + 1 < rhs.first
    }


    /// Compare two intervals.
    /// Overlapping *or abutting* intervals are considered equal.
    fn mergecmp(self, rhs: Interval) -> cmp::Ordering {
        if self.is_strictly_before(rhs) {
            Ordering::Less
        } else if rhs.is_strictly_before(self) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }


    /// Return whether self is mergeable with rhs.
    fn mergeable(self, rhs: Interval) -> bool {
        self.mergecmp(rhs) == Ordering::Equal
    }


    /// Return whether self contains a code point \p cp.
    pub fn contains(self, cp: CodePoint) -> bool {
        self.first <= cp && cp <= self.last
    }


    /// Return whether self overlaps 'other'.
    /// Overlaps means that we share at least one code point with 'other'.
    pub fn overlaps(self, other: Interval) -> bool {
        !self.is_before(other) && !other.is_before(self)
    }


    /// Return the interval of codepoints.
    pub fn codepoints(self) -> core::ops::Range<u32> {
        debug_assert!(self.last + 1 > self.last, "Overflow");
        self.first..(self.last + 1)
    }


    /// Return the number of contained code points.
    pub fn count_codepoints(self) -> usize {
        (self.last - self.first + 1) as usize
    }
}

pub(crate) fn interval_contains(interval: &[Interval], cp: u32) -> bool {
    interval.binary_search_by(|iv| iv.compare(cp)).is_ok()
}
