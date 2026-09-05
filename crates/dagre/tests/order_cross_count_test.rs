//! Port of test/order/cross-count-test.ts.

use dagre::{
	graph::{Graph, NodeIdx},
	order::cross_count,
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

fn vs(g: &Graph<GraphLabel, NodeLabel, EdgeLabel>, s: &[&str]) -> Vec<NodeIdx> {
	s.iter()
		.map(|x| {
			g.node_idx(x)
				.unwrap_or_else(|| panic!("no node named {x}"))
		})
		.collect()
}

#[test]
fn empty_layering_is_zero() {
	let g = mk();
	assert_eq!(cross_count(&g, &[]), 0);
}

#[test]
fn no_crossings_is_zero() {
	let mut g = mk();
	g.set_edge_default("a1", "b1");
	g.set_edge_default("a2", "b2");
	assert_eq!(
		cross_count(&g, &[vs(&g, &["a1", "a2"]), vs(&g, &["b1", "b2"])]),
		0
	);
}

#[test]
fn one_crossing() {
	let mut g = mk();
	g.set_edge_default("a1", "b1");
	g.set_edge_default("a2", "b2");
	assert_eq!(
		cross_count(&g, &[vs(&g, &["a1", "a2"]), vs(&g, &["b2", "b1"])]),
		1
	);
}

#[test]
fn weighted_crossing() {
	let mut g = mk();
	g.set_edge(
		"a1",
		"b1",
		EdgeLabel {
			weight: 2.0,
			..Default::default()
		},
	);
	g.set_edge(
		"a2",
		"b2",
		EdgeLabel {
			weight: 3.0,
			..Default::default()
		},
	);
	assert_eq!(
		cross_count(&g, &[vs(&g, &["a1", "a2"]), vs(&g, &["b2", "b1"])]),
		6
	);
}

#[test]
fn across_layers() {
	let mut g = mk();
	g.set_path(&["a1", "b1", "c1"]);
	g.set_path(&["a2", "b2", "c2"]);
	assert_eq!(
		cross_count(
			&g,
			&[
				vs(&g, &["a1", "a2"]),
				vs(&g, &["b2", "b1"]),
				vs(&g, &["c1", "c2"])
			]
		),
		2
	);
}

#[test]
fn works_for_graph_1() {
	let mut g = mk();
	g.set_path(&["a", "b", "c"]);
	g.set_path(&["d", "e", "c"]);
	g.set_path(&["a", "f", "i"]);
	g.set_edge_default("a", "e");
	assert_eq!(
		cross_count(
			&g,
			&[
				vs(&g, &["a", "d"]),
				vs(&g, &["b", "e", "f"]),
				vs(&g, &["c", "i"])
			]
		),
		1
	);
	assert_eq!(
		cross_count(
			&g,
			&[
				vs(&g, &["d", "a"]),
				vs(&g, &["e", "b", "f"]),
				vs(&g, &["c", "i"])
			]
		),
		0
	);
}
