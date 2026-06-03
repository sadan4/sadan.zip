//! Port of test/order/add-subgraph-constraints-test.ts.

use std::string::ToString;

use dagre::{
	graph::{Edge, Graph, GraphOpts},
	order::add_subgraph_constraints,
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	Graph::with_opts(GraphOpts::directed().compound())
}

fn vs(s: &[&str]) -> Vec<String> {
	s.iter()
		.map(ToString::to_string)
		.collect()
}

#[test]
fn flat_node_set_changes_nothing() {
	let mut g = mk();
	for v in ["a", "b", "c", "d"] {
		g.set_node(v.to_string(), NodeLabel::default());
	}
	let mut cg: Graph<(), (), ()> = Graph::new();
	add_subgraph_constraints(&g, &mut cg, &vs(&["a", "b", "c", "d"]));
	assert_eq!(cg.node_count(), 0);
	assert_eq!(cg.edge_count(), 0);
}

#[test]
fn contiguous_subgraph_no_constraint() {
	let mut g = mk();
	for v in ["a", "b", "c"] {
		g.set_parent(v, Some("sg"));
	}
	let mut cg: Graph<(), (), ()> = Graph::new();
	add_subgraph_constraints(&g, &mut cg, &vs(&["a", "b", "c"]));
	assert_eq!(cg.edge_count(), 0);
}

#[test]
fn adds_constraint_for_adjacent_different_parents() {
	let mut g = mk();
	g.set_parent("a", Some("sg1"));
	g.set_parent("b", Some("sg2"));
	let mut cg: Graph<(), (), ()> = Graph::new();
	add_subgraph_constraints(&g, &mut cg, &vs(&["a", "b"]));
	let edges = cg.edges();
	assert_eq!(edges.len(), 1);
	assert_eq!((edges[0].v.as_str(), edges[0].w.as_str()), ("sg1", "sg2"));
}

#[test]
fn works_for_multiple_levels() {
	let mut g = mk();
	for v in ["a", "b", "c", "d", "e", "f", "g", "h"] {
		g.set_node(v.to_string(), NodeLabel::default());
	}
	g.set_parent("b", Some("sg2"));
	g.set_parent("sg2", Some("sg1"));
	g.set_parent("c", Some("sg1"));
	g.set_parent("d", Some("sg3"));
	g.set_parent("sg3", Some("sg1"));
	g.set_parent("f", Some("sg4"));
	g.set_parent("g", Some("sg5"));
	g.set_parent("sg5", Some("sg4"));
	let mut cg: Graph<(), (), ()> = Graph::new();
	add_subgraph_constraints(
		&g,
		&mut cg,
		&vs(&["a", "b", "c", "d", "e", "f", "g", "h"]),
	);
	let mut edges: Vec<Edge> = cg.edges();
	edges.sort_by(|a, b| a.v.cmp(&b.v));
	assert_eq!(edges.len(), 2);
	let pairs: Vec<(String, String)> = edges
		.into_iter()
		.map(|e| (e.v, e.w))
		.collect();
	assert_eq!(
		pairs,
		vec![("sg1".into(), "sg4".into()), ("sg2".into(), "sg3".into())]
	);
}
