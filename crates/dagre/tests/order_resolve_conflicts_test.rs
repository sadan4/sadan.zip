//! Port of test/order/resolve-conflicts-test.ts.

use std::string::ToString;

use dagre::{
	graph::Graph,
	order::{resolve_conflicts, BarycenterEntry, ResolvedEntry},
};

fn bc(v: &str, b: Option<f64>, w: Option<f64>) -> BarycenterEntry {
	BarycenterEntry {
		v: v.into(),
		barycenter: b,
		weight: w,
	}
}

fn vs(s: &[&str]) -> Vec<String> {
	s.iter()
		.map(ToString::to_string)
		.collect()
}

fn sort_by_first(mut r: Vec<ResolvedEntry>) -> Vec<ResolvedEntry> {
	r.sort_by(|a, b| a.vs[0].cmp(&b.vs[0]));
	r
}

#[test]
fn no_constraints_returns_unchanged() {
	let cg: Graph<(), (), ()> = Graph::new();
	let input =
		vec![bc("a", Some(2.0), Some(3.0)), bc("b", Some(1.0), Some(2.0))];
	let r = sort_by_first(resolve_conflicts(&input, &cg));
	assert_eq!(r.len(), 2);
	assert_eq!(r[0].vs, vs(&["a"]));
	assert_eq!(r[0].barycenter, Some(2.0));
	assert_eq!(r[0].weight, Some(3.0));
	assert_eq!(r[1].vs, vs(&["b"]));
	assert_eq!(r[1].barycenter, Some(1.0));
	assert_eq!(r[1].weight, Some(2.0));
}

#[test]
fn no_conflicts_returns_unchanged() {
	let mut cg: Graph<(), (), ()> = Graph::new();
	cg.set_edge("b", "a", ());
	let input =
		vec![bc("a", Some(2.0), Some(3.0)), bc("b", Some(1.0), Some(2.0))];
	let r = sort_by_first(resolve_conflicts(&input, &cg));
	assert_eq!(r.len(), 2);
}

#[test]
fn coalesces_on_conflict() {
	let mut cg: Graph<(), (), ()> = Graph::new();
	cg.set_edge("a", "b", ());
	let input =
		vec![bc("a", Some(2.0), Some(3.0)), bc("b", Some(1.0), Some(2.0))];
	let r = resolve_conflicts(&input, &cg);
	assert_eq!(r.len(), 1);
	let res = &r[0];
	assert_eq!(res.vs, vs(&["a", "b"]));
	assert_eq!(res.i, 0);
	assert_eq!(res.barycenter, Some((3.0 * 2.0 + 2.0 * 1.0) / 5.0));
	assert_eq!(res.weight, Some(5.0));
}

#[test]
fn coalesces_on_path_constraint() {
	let mut cg: Graph<(), (), ()> = Graph::new();
	cg.set_path(&["a", "b", "c", "d"]);
	let input = vec![
		bc("a", Some(4.0), Some(1.0)),
		bc("b", Some(3.0), Some(1.0)),
		bc("c", Some(2.0), Some(1.0)),
		bc("d", Some(1.0), Some(1.0)),
	];
	let r = resolve_conflicts(&input, &cg);
	assert_eq!(r.len(), 1);
	let res = &r[0];
	assert_eq!(res.vs, vs(&["a", "b", "c", "d"]));
	assert_eq!(res.i, 0);
	assert_eq!(res.barycenter, Some(2.5));
	assert_eq!(res.weight, Some(4.0));
}

#[test]
fn does_nothing_when_no_barycenter_or_constraint() {
	let cg: Graph<(), (), ()> = Graph::new();
	let input = vec![bc("a", None, None), bc("b", Some(1.0), Some(2.0))];
	let r = sort_by_first(resolve_conflicts(&input, &cg));
	assert_eq!(r.len(), 2);
	assert_eq!(r[0].vs, vs(&["a"]));
	assert_eq!(r[0].barycenter, None);
	assert_eq!(r[1].vs, vs(&["b"]));
	assert_eq!(r[1].barycenter, Some(1.0));
}

#[test]
fn ignores_edges_unrelated_to_entries() {
	let mut cg: Graph<(), (), ()> = Graph::new();
	cg.set_edge("c", "d", ());
	let input =
		vec![bc("a", Some(2.0), Some(3.0)), bc("b", Some(1.0), Some(2.0))];
	let r = sort_by_first(resolve_conflicts(&input, &cg));
	assert_eq!(r.len(), 2);
}
