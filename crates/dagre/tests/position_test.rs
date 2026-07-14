//! Port of test/position-test.ts.
//!
//! Coordinates here are exact values computed deterministically by the
//! algorithm, so exact float equality is the intended assertion.
#![allow(clippy::float_cmp)]

use dagre::{
	graph::{Graph, GraphOpts},
	position,
	types::{EdgeLabel, GraphLabel, NodeLabel, RankAlign},
};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
		Graph::with_opts(GraphOpts::directed().compound());
	g.set_graph(GraphLabel {
		ranksep: Some(50.0),
		nodesep: Some(50.0),
		edgesep: Some(10.0),
		..Default::default()
	});
	g
}

fn n(w: f64, h: f64, rank: i32, order: usize) -> NodeLabel {
	NodeLabel {
		width: w,
		height: h,
		rank: Some(rank),
		order: Some(order),
		..Default::default()
	}
}

#[test]
fn respects_ranksep() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.ranksep = Some(1000.0);
	}
	g.set_node("a", n(50.0, 100.0, 0, 0));
	g.set_node("b", n(50.0, 80.0, 1, 0));
	g.set_edge_default("a", "b");
	position::position(&mut g);
	assert_eq!(g.node("b").unwrap().y, Some(100.0 + 1000.0 + 80.0 / 2.0));
}

#[test]
fn uses_largest_height_per_rank_with_ranksep() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.ranksep = Some(1000.0);
	}
	g.set_node("a", n(50.0, 100.0, 0, 0));
	g.set_node("b", n(50.0, 80.0, 0, 1));
	g.set_node("c", n(50.0, 90.0, 1, 0));
	g.set_edge_default("a", "c");
	position::position(&mut g);
	assert_eq!(g.node("a").unwrap().y, Some(50.0));
	assert_eq!(g.node("b").unwrap().y, Some(50.0));
	assert_eq!(g.node("c").unwrap().y, Some(100.0 + 1000.0 + 90.0 / 2.0));
}

#[test]
fn respects_nodesep() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.nodesep = Some(1000.0);
	}
	g.set_node("a", n(50.0, 100.0, 0, 0));
	g.set_node("b", n(70.0, 80.0, 0, 1));
	position::position(&mut g);
	let xa = g.node("a").unwrap().x.unwrap();
	let xb = g.node("b").unwrap().x.unwrap();
	assert_eq!(xb, xa + 50.0 / 2.0 + 1000.0 + 70.0 / 2.0);
}

#[test]
fn does_not_position_subgraph_node() {
	let mut g = mk();
	g.set_node("a", n(50.0, 50.0, 0, 0));
	g.set_node("sg1", NodeLabel::default());
	g.set_parent("a", Some("sg1"));
	position::position(&mut g);
	assert_eq!(g.node("sg1").unwrap().x, None);
	assert_eq!(g.node("sg1").unwrap().y, None);
}

#[test]
fn rankalign_top() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.rank_align = Some(RankAlign::Top);
	}
	g.set_node("a", n(50.0, 100.0, 0, 0));
	g.set_node("b", n(50.0, 60.0, 0, 1));
	position::position(&mut g);
	assert_eq!(g.node("a").unwrap().y, Some(100.0 / 2.0));
	assert_eq!(g.node("b").unwrap().y, Some(60.0 / 2.0));
}

#[test]
fn rankalign_bottom() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.rank_align = Some(RankAlign::Bottom);
	}
	g.set_node("a", n(50.0, 100.0, 0, 0));
	g.set_node("b", n(50.0, 60.0, 0, 1));
	position::position(&mut g);
	assert_eq!(g.node("a").unwrap().y, Some(100.0 - 100.0 / 2.0));
	assert_eq!(g.node("b").unwrap().y, Some(100.0 - 60.0 / 2.0));
}

#[test]
fn rankalign_center() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.rank_align = Some(RankAlign::Center);
	}
	g.set_node("a", n(50.0, 100.0, 0, 0));
	g.set_node("b", n(50.0, 60.0, 0, 1));
	position::position(&mut g);
	assert_eq!(g.node("a").unwrap().y, Some(100.0 / 2.0));
	assert_eq!(g.node("b").unwrap().y, Some(100.0 / 2.0));
}
