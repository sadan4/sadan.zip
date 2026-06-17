//! Port of test/order/barycenter-test.ts.

use dagre::{
	graph::Graph,
	order::{BarycenterEntry, barycenter},
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_default_edge_label(|_| EdgeLabel {
		weight: 1.0,
		..Default::default()
	});
	g
}

fn entry(v: &str, b: Option<f64>, w: Option<f64>) -> BarycenterEntry {
	BarycenterEntry {
		v: v.into(),
		barycenter: b,
		weight: w,
	}
}

#[test]
fn no_predecessor_gives_undefined_barycenter() {
	let mut g = mk();
	g.set_node("x", NodeLabel::default());
	let r = barycenter(&g, &["x".into()]);
	assert_eq!(r, vec![entry("x", None, None)]);
}

#[test]
fn sole_predecessor_position() {
	let mut g = mk();
	g.set_node(
		"a",
		NodeLabel {
			order: Some(2),
			..Default::default()
		},
	);
	g.set_edge_default("a", "x");
	let r = barycenter(&g, &["x".into()]);
	assert_eq!(r, vec![entry("x", Some(2.0), Some(1.0))]);
}

#[test]
fn average_of_multiple_predecessors() {
	let mut g = mk();
	g.set_node(
		"a",
		NodeLabel {
			order: Some(2),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			order: Some(4),
			..Default::default()
		},
	);
	g.set_edge_default("a", "x");
	g.set_edge_default("b", "x");
	let r = barycenter(&g, &["x".into()]);
	assert_eq!(r, vec![entry("x", Some(3.0), Some(2.0))]);
}

#[test]
fn takes_edge_weight_into_account() {
	let mut g = mk();
	g.set_node(
		"a",
		NodeLabel {
			order: Some(2),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			order: Some(4),
			..Default::default()
		},
	);
	g.set_edge(
		"a",
		"x",
		EdgeLabel {
			weight: 3.0,
			..Default::default()
		},
	);
	g.set_edge_default("b", "x");
	let r = barycenter(&g, &["x".into()]);
	assert_eq!(r, vec![entry("x", Some(2.5), Some(4.0))]);
}

#[test]
fn computes_per_movable_node() {
	let mut g = mk();
	g.set_node(
		"a",
		NodeLabel {
			order: Some(1),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			order: Some(2),
			..Default::default()
		},
	);
	g.set_node(
		"c",
		NodeLabel {
			order: Some(4),
			..Default::default()
		},
	);
	g.set_edge_default("a", "x");
	g.set_edge_default("b", "x");
	g.set_node("y", NodeLabel::default());
	g.set_edge(
		"a",
		"z",
		EdgeLabel {
			weight: 2.0,
			..Default::default()
		},
	);
	g.set_edge_default("c", "z");
	let r = barycenter(&g, &["x".into(), "y".into(), "z".into()]);
	assert_eq!(r.len(), 3);
	assert_eq!(r[0], entry("x", Some(1.5), Some(2.0)));
	assert_eq!(r[1], entry("y", None, None));
	assert_eq!(r[2], entry("z", Some(2.0), Some(3.0)));
}
