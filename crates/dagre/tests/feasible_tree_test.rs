//! Port of test/rank/feasible-tree-test.ts.

use dagre::{
	graph::Graph,
	rank::feasible_tree::{self, Tree},
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

/// The tree shares `g`'s node index space but carries no name side-table, so
/// names have to be resolved through `g` in both directions.
fn nbrs(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	tree: &Tree,
	v: &str,
) -> Vec<String> {
	let idx = g
		.node_idx(v)
		.unwrap_or_else(|| panic!("no node named {v}"));
	let mut out = g.names_of(&tree.neighbors(idx).unwrap_or_default());
	out.sort();
	out
}

#[test]
fn trivial_two_node_graph() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(0),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			rank: Some(1),
			..Default::default()
		},
	);
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			minlen: 1,
			..Default::default()
		},
	);

	let tree = feasible_tree::build(&mut g);
	assert_eq!(
		g.node("b").unwrap().rank.unwrap(),
		g.node("a").unwrap().rank.unwrap() + 1
	);
	assert_eq!(nbrs(&g, &tree, "a"), ["b"]);
}

#[test]
fn shortens_slack_by_pulling_up() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(0),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			rank: Some(1),
			..Default::default()
		},
	);
	g.set_node(
		"c",
		NodeLabel {
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_node(
		"d",
		NodeLabel {
			rank: Some(2),
			..Default::default()
		},
	);
	let el = || EdgeLabel {
		minlen: 1,
		..Default::default()
	};
	g.set_edge("a", "b", el());
	g.set_edge("b", "c", el());
	g.set_edge("a", "d", el());

	let tree = feasible_tree::build(&mut g);
	let ra = g.node("a").unwrap().rank.unwrap();
	let rb = g.node("b").unwrap().rank.unwrap();
	let rc = g.node("c").unwrap().rank.unwrap();
	let rd = g.node("d").unwrap().rank.unwrap();
	assert_eq!(rb, ra + 1);
	assert_eq!(rc, rb + 1);
	assert_eq!(rd, ra + 1);

	assert_eq!(nbrs(&g, &tree, "a"), ["b", "d"]);
	assert_eq!(nbrs(&g, &tree, "b"), ["a", "c"]);
	assert_eq!(nbrs(&g, &tree, "c"), ["b"]);
	assert_eq!(nbrs(&g, &tree, "d"), ["a"]);
}

#[test]
fn shortens_slack_by_pulling_down() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			rank: Some(0),
			..Default::default()
		},
	);
	g.set_node(
		"c",
		NodeLabel {
			rank: Some(2),
			..Default::default()
		},
	);
	let el = || EdgeLabel {
		minlen: 1,
		..Default::default()
	};
	g.set_edge("b", "a", el());
	g.set_edge("b", "c", el());

	let tree = feasible_tree::build(&mut g);
	let ra = g.node("a").unwrap().rank.unwrap();
	let rb = g.node("b").unwrap().rank.unwrap();
	let rc = g.node("c").unwrap().rank.unwrap();
	assert_eq!(ra, rb + 1);
	assert_eq!(rc, rb + 1);
	assert_eq!(nbrs(&g, &tree, "a"), ["b"]);
	assert_eq!(nbrs(&g, &tree, "b"), ["a", "c"]);
	assert_eq!(nbrs(&g, &tree, "c"), ["b"]);
}
