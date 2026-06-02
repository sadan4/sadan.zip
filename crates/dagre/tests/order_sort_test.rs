//! Port of test/order/sort-test.ts.

use dagre::order::{sort, ResolvedEntry};

fn entry(
	vs: &[&str],
	i: usize,
	b: Option<f64>,
	w: Option<f64>,
) -> ResolvedEntry {
	ResolvedEntry {
		vs: vs
			.iter()
			.map(|s| s.to_string())
			.collect(),
		i,
		barycenter: b,
		weight: w,
	}
}

fn vs(s: &[&str]) -> Vec<String> {
	s.iter()
		.map(|x| x.to_string())
		.collect()
}

#[test]
fn sorts_by_barycenter() {
	let input = vec![
		entry(&["a"], 0, Some(2.0), Some(3.0)),
		entry(&["b"], 1, Some(1.0), Some(2.0)),
	];
	let r = sort(input, false);
	assert_eq!(r.vs, vs(&["b", "a"]));
	assert_eq!(r.barycenter, Some((2.0 * 3.0 + 1.0 * 2.0) / (3.0 + 2.0)));
	assert_eq!(r.weight, Some(5.0));
}

#[test]
fn sorts_super_nodes() {
	let input = vec![
		entry(&["a", "c", "d"], 0, Some(2.0), Some(3.0)),
		entry(&["b"], 1, Some(1.0), Some(2.0)),
	];
	let r = sort(input, false);
	assert_eq!(r.vs, vs(&["b", "a", "c", "d"]));
	assert_eq!(r.barycenter, Some((2.0 * 3.0 + 1.0 * 2.0) / (3.0 + 2.0)));
	assert_eq!(r.weight, Some(5.0));
}

#[test]
fn biases_left_by_default() {
	let input = vec![
		entry(&["a"], 0, Some(1.0), Some(1.0)),
		entry(&["b"], 1, Some(1.0), Some(1.0)),
	];
	let r = sort(input, false);
	assert_eq!(r.vs, vs(&["a", "b"]));
	assert_eq!(r.barycenter, Some(1.0));
	assert_eq!(r.weight, Some(2.0));
}

#[test]
fn biases_right_with_bias_right_true() {
	let input = vec![
		entry(&["a"], 0, Some(1.0), Some(1.0)),
		entry(&["b"], 1, Some(1.0), Some(1.0)),
	];
	let r = sort(input, true);
	assert_eq!(r.vs, vs(&["b", "a"]));
	assert_eq!(r.barycenter, Some(1.0));
	assert_eq!(r.weight, Some(2.0));
}

#[test]
fn handles_unsortable_nodes() {
	let input = vec![
		entry(&["a"], 0, Some(2.0), Some(1.0)),
		entry(&["b"], 1, Some(6.0), Some(1.0)),
		entry(&["c"], 2, None, None),
		entry(&["d"], 3, Some(3.0), Some(1.0)),
	];
	let r = sort(input, false);
	assert_eq!(r.vs, vs(&["a", "d", "c", "b"]));
	assert_eq!(r.barycenter, Some((2.0 + 6.0 + 3.0) / 3.0));
	assert_eq!(r.weight, Some(3.0));
}

#[test]
fn handles_no_barycenters() {
	let input = vec![
		entry(&["a"], 0, None, None),
		entry(&["b"], 3, None, None),
		entry(&["c"], 2, None, None),
		entry(&["d"], 1, None, None),
	];
	let r = sort(input, false);
	assert_eq!(r.vs, vs(&["a", "d", "c", "b"]));
	assert_eq!(r.barycenter, None);
	assert_eq!(r.weight, None);
}

#[test]
fn handles_barycenter_zero() {
	let input = vec![
		entry(&["a"], 0, Some(0.0), Some(1.0)),
		entry(&["b"], 3, None, None),
		entry(&["c"], 2, None, None),
		entry(&["d"], 1, None, None),
	];
	let r = sort(input, false);
	assert_eq!(r.vs, vs(&["a", "d", "c", "b"]));
	assert_eq!(r.barycenter, Some(0.0));
	assert_eq!(r.weight, Some(1.0));
}
