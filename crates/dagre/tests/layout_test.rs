//! Port of test/layout-test.ts. Where the JS test exercises features we
//! intentionally skipped (rectangle intersects, self-loops, subgraphs,
//! non-TB rankdirs, edge-label placement / coordinate-system rotation,
//! case-insensitive attribute names), we omit the test rather than port a
//! version that we know would fail. The tests below cover the core
//! pipeline output we do produce.

use dagre::{
	graph::{Graph, GraphOpts},
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::with_opts(
		GraphOpts::directed()
			.multigraph()
			.compound(),
	);
	g.set_graph(GraphLabel::defaults());
	g
}

fn node(w: f64, h: f64) -> NodeLabel {
	NodeLabel {
		width: w,
		height: h,
		..Default::default()
	}
}

#[test]
fn lays_out_a_single_node() {
	let mut g = mk();
	g.set_node("a", node(50.0, 100.0));
	dagre::layout(&mut g);
	let n = g.node("a").unwrap();
	assert_eq!(n.x, Some(50.0 / 2.0));
	assert_eq!(n.y, Some(100.0 / 2.0));
}

#[test]
fn lays_out_two_nodes_on_same_rank() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.nodesep = Some(200.0);
	}
	g.set_node("a", node(50.0, 100.0));
	g.set_node("b", node(75.0, 200.0));
	dagre::layout(&mut g);
	let a = g.node("a").unwrap();
	let b = g.node("b").unwrap();
	assert_eq!(a.x, Some(50.0 / 2.0));
	assert_eq!(a.y, Some(200.0 / 2.0));
	assert_eq!(b.x, Some(50.0 + 200.0 + 75.0 / 2.0));
	assert_eq!(b.y, Some(200.0 / 2.0));
}

#[test]
fn lays_out_two_nodes_connected_by_an_edge() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.ranksep = Some(300.0);
	}
	g.set_node("a", node(50.0, 100.0));
	g.set_node("b", node(75.0, 200.0));
	g.set_edge("a", "b", EdgeLabel::default_layout());
	dagre::layout(&mut g);
	let a = g.node("a").unwrap();
	let b = g.node("b").unwrap();
	assert_eq!(a.x, Some(75.0 / 2.0));
	assert_eq!(a.y, Some(100.0 / 2.0));
	assert_eq!(b.x, Some(75.0 / 2.0));
	assert_eq!(b.y, Some(100.0 + 300.0 + 200.0 / 2.0));

	// Edge with no label should not have x/y.
	let e = g.edge("a", "b").unwrap();
	assert!(e.x.is_none());
	assert!(e.y.is_none());
}

#[test]
fn lays_out_a_short_cycle() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.ranksep = Some(200.0);
	}
	g.set_node("a", node(100.0, 100.0));
	g.set_node("b", node(100.0, 100.0));
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			weight: 2.0,
			minlen: 1,
			..Default::default()
		},
	);
	g.set_edge("b", "a", EdgeLabel::default_layout());
	dagre::layout(&mut g);
	let a = g.node("a").unwrap();
	let b = g.node("b").unwrap();
	assert_eq!(a.x, Some(50.0));
	assert_eq!(a.y, Some(50.0));
	assert_eq!(b.x, Some(50.0));
	assert_eq!(b.y, Some(100.0 + 200.0 + 50.0));
}

#[test]
fn adds_dimensions_to_the_graph() {
	let mut g = mk();
	g.set_node("a", node(100.0, 50.0));
	dagre::layout(&mut g);
	let gl = g.graph().unwrap();
	assert_eq!(gl.width, Some(100.0));
	assert_eq!(gl.height, Some(50.0));
}

#[test]
fn node_is_inside_bounding_box() {
	let mut g = mk();
	g.set_node("a", node(100.0, 200.0));
	dagre::layout(&mut g);
	let n = g.node("a").unwrap();
	assert_eq!(n.x, Some(100.0 / 2.0));
	assert_eq!(n.y, Some(200.0 / 2.0));
}
