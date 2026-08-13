#![allow(clippy::suboptimal_flops)]
//! Port of test/util-test.ts.
//!
//! The JS tests use dynamic `{foo: "bar"}` labels. Where that doesn't
//! translate, we use the typed `NodeLabel`/`EdgeLabel` and assert on the
//! specific numeric fields that drive layout (`weight`, `minlen`, `rank`,
//! `order`).
//!
//! Layout values here are exact integers/halves computed deterministically
//! by the algorithm, so exact float equality is the intended assertion.
#![allow(clippy::float_cmp)]

use dagre::{
	graph::{Graph, GraphOpts},
	types::{EdgeLabel, GraphLabel, NodeLabel, Point},
	util,
};

fn mg() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	Graph::with_opts(GraphOpts::directed().multigraph())
}

fn mc() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	Graph::with_opts(
		GraphOpts::directed()
			.multigraph()
			.compound(),
	)
}

// ---------- simplify -----------------------------------------------------

#[test]
fn simplify_copies_a_graph_with_no_multi_edges() {
	let mut g = mg();
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			weight: 1.0,
			minlen: 1,
			..Default::default()
		},
	);
	let g2 = util::simplify(&g);
	let e = g2.edge("a", "b").unwrap();
	assert_eq!(e.weight, 1.0);
	assert_eq!(e.minlen, 1);
	assert_eq!(g2.edge_count(), 1);
}

#[test]
fn simplify_collapses_multi_edges() {
	let mut g = mg();
	g.set_edge_named(
		"a",
		"b",
		EdgeLabel {
			weight: 1.0,
			minlen: 1,
			..Default::default()
		},
		None,
	);
	g.set_edge_named(
		"a",
		"b",
		EdgeLabel {
			weight: 2.0,
			minlen: 2,
			..Default::default()
		},
		Some("multi".into()),
	);
	let g2 = util::simplify(&g);
	assert!(!g2.is_multigraph());
	let e = g2.edge("a", "b").unwrap();
	assert_eq!(e.weight, 3.0);
	assert_eq!(e.minlen, 2);
	assert_eq!(g2.edge_count(), 1);
}

#[test]
fn simplify_copies_the_graph_label() {
	let mut g = mg();
	g.set_graph(GraphLabel {
		ranksep: Some(42.0),
		..Default::default()
	});
	let g2 = util::simplify(&g);
	assert_eq!(g2.graph().unwrap().ranksep, Some(42.0));
}

// ---------- asNonCompoundGraph -------------------------------------------

#[test]
fn as_non_compound_copies_all_nodes() {
	let mut g = mc();
	g.set_node(
		"a",
		NodeLabel {
			width: 5.0,
			..Default::default()
		},
	);
	g.set_node("b", NodeLabel::default());
	let g2 = util::as_non_compound_graph(&g);
	assert_eq!(g2.node("a").unwrap().width, 5.0);
	assert!(g2.has_node("b"));
}

#[test]
fn as_non_compound_copies_all_edges_including_named() {
	let mut g = mc();
	g.set_edge_named(
		"a",
		"b",
		EdgeLabel {
			weight: 1.0,
			..Default::default()
		},
		None,
	);
	g.set_edge_named(
		"a",
		"b",
		EdgeLabel {
			weight: 2.0,
			..Default::default()
		},
		Some("multi".into()),
	);
	let g2 = util::as_non_compound_graph(&g);
	assert_eq!(g2.edge("a", "b").unwrap().weight, 1.0);
	assert_eq!(
		g2.edge_full("a", "b", Some("multi"))
			.unwrap()
			.weight,
		2.0
	);
}

#[test]
fn as_non_compound_skips_compound_parents() {
	let mut g = mc();
	g.set_node("a", NodeLabel::default());
	g.set_node("sg1", NodeLabel::default());
	g.set_parent("a", Some("sg1"));
	let g2 = util::as_non_compound_graph(&g);
	assert!(!g2.is_compound());
	assert!(!g2.has_node("sg1"));
}

#[test]
fn as_non_compound_copies_graph_label() {
	let mut g = mc();
	g.set_graph(GraphLabel {
		nodesep: Some(7.0),
		..Default::default()
	});
	let g2 = util::as_non_compound_graph(&g);
	assert_eq!(g2.graph().unwrap().nodesep, Some(7.0));
}

// ---------- successorWeights / predecessorWeights ------------------------

fn mk_weight_graph() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g = mg();
	g.set_edge_named(
		"a",
		"b",
		EdgeLabel {
			weight: 2.0,
			..Default::default()
		},
		None,
	);
	g.set_edge_named(
		"b",
		"c",
		EdgeLabel {
			weight: 1.0,
			..Default::default()
		},
		None,
	);
	g.set_edge_named(
		"b",
		"c",
		EdgeLabel {
			weight: 2.0,
			..Default::default()
		},
		Some("multi".into()),
	);
	g.set_edge_named(
		"b",
		"d",
		EdgeLabel {
			weight: 1.0,
			..Default::default()
		},
		Some("multi".into()),
	);
	g
}

#[test]
fn successor_weights_sums_per_destination() {
	let g = mk_weight_graph();
	let m = util::successor_weights(&g);
	assert_eq!(m.get("a").unwrap().get("b").copied(), Some(2.0));
	assert_eq!(m.get("b").unwrap().get("c").copied(), Some(3.0));
	assert_eq!(m.get("b").unwrap().get("d").copied(), Some(1.0));
	assert!(m.get("c").unwrap().is_empty());
	assert!(m.get("d").unwrap().is_empty());
}

#[test]
fn predecessor_weights_sums_per_source() {
	let g = mk_weight_graph();
	let m = util::predecessor_weights(&g);
	assert!(m.get("a").unwrap().is_empty());
	assert_eq!(m.get("b").unwrap().get("a").copied(), Some(2.0));
	assert_eq!(m.get("c").unwrap().get("b").copied(), Some(3.0));
	assert_eq!(m.get("d").unwrap().get("b").copied(), Some(1.0));
}

// ---------- intersectRect ------------------------------------------------

fn unit_rect() -> NodeLabel {
	NodeLabel {
		width: 1.0,
		height: 1.0,
		x: Some(0.0),
		y: Some(0.0),
		..Default::default()
	}
}

fn expect_intersects(rect: &NodeLabel, point: Point) {
	let cross = util::intersect_rect(rect, point);
	let rx = rect.x.unwrap();
	let ry = rect.y.unwrap();
	if cross.x != point.x {
		let m = (cross.y - point.y) / (cross.x - point.x);
		assert!(
			((cross.y - ry) - m * (cross.x - rx)).abs() < 1e-9,
			"slope mismatch for point ({}, {}): cross=({}, {})",
			point.x,
			point.y,
			cross.x,
			cross.y
		);
	}
}

fn expect_touches_border(rect: &NodeLabel, point: Point) {
	let cross = util::intersect_rect(rect, point);
	let rx = rect.x.unwrap();
	let ry = rect.y.unwrap();
	if (rx - cross.x).abs() != rect.width / 2.0 {
		assert_eq!((ry - cross.y).abs(), rect.height / 2.0);
	}
}

#[test]
fn intersect_rect_slope_through_center() {
	let r = unit_rect();
	let pts = [
		(2.0, 6.0),
		(2.0, -6.0),
		(6.0, 2.0),
		(-6.0, 2.0),
		(5.0, 0.0),
		(0.0, 5.0),
	];
	for (x, y) in pts {
		expect_intersects(&r, Point { x, y });
	}
}

#[test]
fn intersect_rect_touches_border() {
	let r = unit_rect();
	let pts = [
		(2.0, 6.0),
		(2.0, -6.0),
		(6.0, 2.0),
		(-6.0, 2.0),
		(5.0, 0.0),
		(0.0, 5.0),
	];
	for (x, y) in pts {
		expect_touches_border(&r, Point { x, y });
	}
}

#[test]
#[should_panic(
	expected = "Not possible to find intersection inside of the rectangle"
)]
fn intersect_rect_panics_at_center() {
	let r = unit_rect();
	let _ = util::intersect_rect(&r, Point { x: 0.0, y: 0.0 });
}

// ---------- buildLayerMatrix --------------------------------------------

#[test]
fn build_layer_matrix_groups_by_rank_and_order() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(0),
			order: Some(0),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			rank: Some(0),
			order: Some(1),
			..Default::default()
		},
	);
	g.set_node(
		"c",
		NodeLabel {
			rank: Some(1),
			order: Some(0),
			..Default::default()
		},
	);
	g.set_node(
		"d",
		NodeLabel {
			rank: Some(1),
			order: Some(1),
			..Default::default()
		},
	);
	g.set_node(
		"e",
		NodeLabel {
			rank: Some(2),
			order: Some(0),
			..Default::default()
		},
	);
	let layers = util::build_layer_matrix(&g);
	assert_eq!(layers.len(), 3);
	assert_eq!(layers[0], vec!["a".to_string(), "b".to_string()]);
	assert_eq!(layers[1], vec!["c".to_string(), "d".to_string()]);
	assert_eq!(layers[2], vec!["e".to_string()]);
}

// ---------- normalizeRanks ----------------------------------------------

#[test]
fn normalize_ranks_shifts_to_nonneg_with_zero_min() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(3),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_node(
		"c",
		NodeLabel {
			rank: Some(4),
			..Default::default()
		},
	);
	util::normalize_ranks(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(1));
	assert_eq!(g.node("b").unwrap().rank, Some(0));
	assert_eq!(g.node("c").unwrap().rank, Some(2));
}

#[test]
fn normalize_ranks_handles_negative() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(-3),
			..Default::default()
		},
	);
	g.set_node(
		"b",
		NodeLabel {
			rank: Some(-2),
			..Default::default()
		},
	);
	util::normalize_ranks(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
}

#[test]
fn normalize_ranks_does_not_assign_rank_to_subgraphs_without_one() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
		Graph::with_opts(GraphOpts::directed().compound());
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(0),
			..Default::default()
		},
	);
	g.set_node("sg", NodeLabel::default()); // no rank
	g.set_parent("a", Some("sg"));
	util::normalize_ranks(&mut g);
	assert_eq!(g.node("sg").unwrap().rank, None);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
}

// ---------- removeEmptyRanks --------------------------------------------

#[test]
fn remove_empty_ranks_removes_border_empty_ranks() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_graph(GraphLabel {
		node_rank_factor: Some(4.0),
		..Default::default()
	});
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
			rank: Some(4),
			..Default::default()
		},
	);
	util::remove_empty_ranks(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
}

#[test]
fn remove_empty_ranks_keeps_non_border_ranks() {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_graph(GraphLabel {
		node_rank_factor: Some(4.0),
		..Default::default()
	});
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
			rank: Some(8),
			..Default::default()
		},
	);
	util::remove_empty_ranks(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(2));
}

// ---------- range -------------------------------------------------------

#[test]
fn range_to_limit() {
	let r = util::range0(4);
	assert_eq!(r, vec![0, 1, 2, 3]);
	let sum: i32 = r.iter().sum();
	assert_eq!(sum, 6);
}

#[test]
fn range_with_start() {
	let r = util::range(2, 4, 1);
	assert_eq!(r.len(), 2);
	let sum: i32 = r.iter().sum();
	assert_eq!(sum, 5);
}

#[test]
fn range_with_negative_step() {
	let r = util::range(5, -1, -1);
	assert_eq!(r[0], 5);
	assert_eq!(r.last().copied(), Some(0));
	assert_eq!(r.len(), 6);
}

// ---------- uniqueId ----------------------------------------------------

#[test]
fn unique_id_format_and_distinct() {
	let id = util::unique_id("_root");
	assert!(id.starts_with("_root"));
	let a = util::unique_id("name");
	let b = util::unique_id("name");
	let c = util::unique_id("name");
	assert_ne!(a, b);
	assert_ne!(b, c);
	let nid = util::unique_id("99");
	assert!(nid.starts_with("99"));
}
