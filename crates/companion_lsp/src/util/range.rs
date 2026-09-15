use std::mem;

use tower_lsp_server::ls_types::Range;

/// Returns the overlap between `r1` and `r2`, or [`None`] if they are
/// disjoint.
///
/// Ranges that merely touch (one's `end` equals the other's `start`)
/// intersect in the empty range at that position.
pub fn intersect(mut r1: Range, mut r2: Range) -> Option<Range> {
	// order by `start` so `r1` is the earlier range
	if r2.start < r1.start {
		mem::swap(&mut r1, &mut r2);
	}
	if r1.end < r2.start {
		// no intersection
		None
	} else {
		Some(Range {
			start: r1.start.max(r2.start),
			end: r1.end.min(r2.end),
		})
	}
}

/// Returns the smallest range containing both `r1` and `r2`.
#[cfg_attr(not(test), expect(unused))]
pub fn union(r1: Range, r2: Range) -> Range {
	Range {
		start: r1.start.min(r2.start),
		end: r1.end.max(r2.end),
	}
}

#[cfg(test)]
mod tests {
	use tower_lsp_server::ls_types::{Position, Range};

	use super::{intersect, union};

	/// `r(1, 2, 3, 4)` == `1:2..3:4`
	fn r(
		start_line: u32,
		start_character: u32,
		end_line: u32,
		end_character: u32,
	) -> Range {
		Range {
			start: Position {
				line: start_line,
				character: start_character,
			},
			end: Position {
				line: end_line,
				character: end_character,
			},
		}
	}

	mod intersect {
		use super::*;

		#[test]
		fn intersect_is_commutative() {
			let cases = [
				(r(0, 0, 0, 10), r(0, 5, 0, 20)),
				(r(0, 0, 0, 10), r(0, 20, 0, 30)),
				(r(1, 0, 5, 0), r(2, 0, 3, 0)),
				(r(0, 0, 0, 0), r(0, 0, 0, 0)),
				(r(0, 5, 1, 5), r(1, 5, 2, 0)),
			];
			for (a, b) in cases {
				assert_eq!(
					intersect(a, b),
					intersect(b, a),
					"intersect({a:?}, {b:?}) is not commutative"
				);
			}
		}

		#[test]
		fn intersect_partial_overlap() {
			assert_eq!(
				intersect(r(0, 0, 0, 10), r(0, 5, 0, 20)),
				Some(r(0, 5, 0, 10))
			);
			assert_eq!(
				intersect(r(0, 5, 0, 20), r(0, 0, 0, 10)),
				Some(r(0, 5, 0, 10))
			);
		}

		#[test]
		fn intersect_multiline_partial_overlap() {
			assert_eq!(
				intersect(r(1, 4, 3, 2), r(2, 0, 9, 9)),
				Some(r(2, 0, 3, 2))
			);
			assert_eq!(
				intersect(r(2, 0, 9, 9), r(1, 4, 3, 2)),
				Some(r(2, 0, 3, 2))
			);
		}

		#[test]
		fn intersect_contained() {
			let outer = r(1, 0, 5, 0);
			let inner = r(2, 3, 3, 4);
			assert_eq!(intersect(outer, inner), Some(inner));
			assert_eq!(intersect(inner, outer), Some(inner));
		}

		#[test]
		fn intersect_identical() {
			let a = r(1, 2, 3, 4);
			assert_eq!(intersect(a, a), Some(a));
		}

		#[test]
		fn intersect_shared_start() {
			assert_eq!(
				intersect(r(0, 0, 0, 5), r(0, 0, 0, 9)),
				Some(r(0, 0, 0, 5))
			);
		}

		#[test]
		fn intersect_shared_end() {
			assert_eq!(
				intersect(r(0, 2, 0, 9), r(0, 6, 0, 9)),
				Some(r(0, 6, 0, 9))
			);
		}

		#[test]
		fn intersect_disjoint_same_line() {
			assert_eq!(intersect(r(0, 0, 0, 5), r(0, 6, 0, 9)), None);
			assert_eq!(intersect(r(0, 6, 0, 9), r(0, 0, 0, 5)), None);
		}

		#[test]
		fn intersect_disjoint_different_lines() {
			assert_eq!(intersect(r(0, 0, 1, 0), r(4, 0, 7, 3)), None);
			assert_eq!(intersect(r(4, 0, 7, 3), r(0, 0, 1, 0)), None);
		}

		#[test]
		fn intersect_never_returns_inverted_range() {
			let cases = [
				(r(0, 0, 0, 5), r(0, 6, 0, 9)),
				(r(0, 6, 0, 9), r(0, 0, 0, 5)),
				(r(1, 0, 2, 0), r(9, 0, 9, 1)),
				(r(9, 0, 9, 1), r(1, 0, 2, 0)),
				(r(0, 0, 3, 0), r(1, 0, 9, 0)),
			];
			for (a, b) in cases {
				if let Some(got) = intersect(a, b) {
					assert!(
						got.start <= got.end,
						"intersect({a:?}, {b:?}) returned inverted \
						{got:?}"
					);
				}
			}
		}

		/// Touching ranges intersect in the empty range where they meet,
		/// rather than reporting no intersection.
		#[test]
		fn intersect_touching_endpoints() {
			assert_eq!(
				intersect(r(0, 0, 0, 5), r(0, 5, 0, 9)),
				Some(r(0, 5, 0, 5))
			);
			assert_eq!(
				intersect(r(0, 5, 0, 9), r(0, 0, 0, 5)),
				Some(r(0, 5, 0, 5))
			);
		}

		#[test]
		fn intersect_empty_range_inside() {
			let empty = r(2, 2, 2, 2);
			assert_eq!(intersect(r(1, 0, 5, 0), empty), Some(empty));
			assert_eq!(intersect(empty, r(1, 0, 5, 0)), Some(empty));
		}

		#[test]
		fn intersect_empty_range_outside() {
			let empty = r(7, 0, 7, 0);
			assert_eq!(intersect(r(1, 0, 5, 0), empty), None);
			assert_eq!(intersect(empty, r(1, 0, 5, 0)), None);
		}
	}

	mod union {
		use super::*;

		#[test]
		fn is_commutative() {
			let cases = [
				(r(0, 0, 0, 10), r(0, 5, 0, 20)),
				(r(0, 0, 0, 10), r(0, 20, 0, 30)),
				(r(1, 0, 5, 0), r(2, 0, 3, 0)),
			];
			for (a, b) in cases {
				assert_eq!(
					union(a, b),
					union(b, a),
					"union({a:?}, {b:?}) is not commutative"
				);
			}
		}

		#[test]
		fn overlapping() {
			assert_eq!(union(r(0, 0, 0, 10), r(0, 5, 0, 20)), r(0, 0, 0, 20));
		}

		#[test]
		fn disjoint_spans_the_gap() {
			assert_eq!(union(r(0, 0, 0, 5), r(3, 1, 4, 0)), r(0, 0, 4, 0));
			assert_eq!(union(r(3, 1, 4, 0), r(0, 0, 0, 5)), r(0, 0, 4, 0));
		}

		#[test]
		fn contained() {
			let outer = r(1, 0, 5, 0);
			assert_eq!(union(outer, r(2, 3, 3, 4)), outer);
			assert_eq!(union(r(2, 3, 3, 4), outer), outer);
		}

		#[test]
		fn identical() {
			let a = r(1, 2, 3, 4);
			assert_eq!(union(a, a), a);
		}
	}
}
