//! Port of test/position/bk-test.ts. We mirror the JS structure: helpers
//! create a graph with the right node-label fields, then call the BK
//! internals exposed via `dagre::position::bk`. Where the JS sets
//! `dummy: true` (any truthy) we use the `Dummy::Edge` variant since our
//! checks are `Option<Dummy>::is_some()`.
//!
//! Coordinates here are exact integers/halves computed deterministically
//! by the algorithm, so exact float equality is the intended assertion.
#![allow(clippy::float_cmp)]

use dagre::{
	graph::{Graph, NodeIdx},
	position::bk::{
		Conflicts,
		NodeMap,
		PositionMap,
		add_conflict,
		align_coordinates,
		balance,
		find_smallest_width_alignment,
		find_type1_conflicts,
		find_type2_conflicts,
		has_conflict,
		horizontal_compaction,
		position_x,
		vertical_alignment,
	},
	types::{Dummy, EdgeLabel, GraphLabel, LabelPos, NodeLabel},
	util::build_layer_matrix,
};
use rustc_hash::FxHashMap;

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_graph(GraphLabel::default());
	g.set_default_edge_label(|_| EdgeLabel::default());
	g
}

fn node(rank: i32, order: usize) -> NodeLabel {
	NodeLabel {
		rank: Some(rank),
		order: Some(order),
		..Default::default()
	}
}

fn n_w(rank: i32, order: usize, width: f64) -> NodeLabel {
	NodeLabel {
		rank: Some(rank),
		order: Some(order),
		width,
		..Default::default()
	}
}

/// Look up a node by the name the test gave it.
fn ix(g: &Graph<GraphLabel, NodeLabel, EdgeLabel>, v: &str) -> NodeIdx {
	g.node_idx(v)
		.unwrap_or_else(|| panic!("no node named {v}"))
}

/// For the conflict-set tests, which have no graph: ids are opaque distinct
/// tokens, so map each single-letter name to a fixed index.
fn n(s: &str) -> NodeIdx {
	NodeIdx(u32::from(s.as_bytes()[0]))
}

/// Graph-free `pmap`, for the alignment tests that only need distinct keys.
fn tmap(pairs: &[(&str, f64)]) -> PositionMap {
	pairs
		.iter()
		.map(|(k, v)| (n(k), *v))
		.collect()
}

fn pmap(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	pairs: &[(&str, f64)],
) -> PositionMap {
	pairs
		.iter()
		.map(|(k, v)| (ix(g, k), *v))
		.collect()
}

fn smap(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	pairs: &[(&str, &str)],
) -> NodeMap<NodeIdx> {
	pairs
		.iter()
		.map(|(k, v)| (ix(g, k), ix(g, v)))
		.collect()
}

// ---------- hasConflict --------------------------------------------------

#[test]
fn has_conflict_either_orientation() {
	let mut c: Conflicts = Conflicts::default();
	add_conflict(&mut c, n("b"), n("a"));
	assert!(has_conflict(&c, n("a"), n("b")));
	assert!(has_conflict(&c, n("b"), n("a")));
}

#[test]
fn has_conflict_multiple_with_same_node() {
	let mut c: Conflicts = Conflicts::default();
	add_conflict(&mut c, n("a"), n("b"));
	add_conflict(&mut c, n("a"), n("c"));
	assert!(has_conflict(&c, n("a"), n("b")));
	assert!(has_conflict(&c, n("a"), n("c")));
}

// ---------- findType1Conflicts ------------------------------------------

fn t1_base() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	g.set_node("b", node(0, 1));
	g.set_node("c", node(1, 0));
	g.set_node("d", node(1, 1));
	g.set_edge_default("a", "d");
	g.set_edge_default("b", "c");
	g
}

#[test]
fn type1_no_conflict_for_uncrossed_edges() {
	let mut g = t1_base();
	g.remove_edge("a", "d");
	g.remove_edge("b", "c");
	g.set_edge_default("a", "c");
	g.set_edge_default("b", "d");
	let layering = build_layer_matrix(&g);
	let c = find_type1_conflicts(&g, &layering);
	assert!(!has_conflict(&c, ix(&g, "a"), ix(&g, "c")));
	assert!(!has_conflict(&c, ix(&g, "b"), ix(&g, "d")));
}

#[test]
fn type1_no_conflict_for_type0_no_dummies() {
	let g = t1_base();
	let layering = build_layer_matrix(&g);
	let c = find_type1_conflicts(&g, &layering);
	assert!(!has_conflict(&c, ix(&g, "a"), ix(&g, "d")));
	assert!(!has_conflict(&c, ix(&g, "b"), ix(&g, "c")));
}

#[test]
fn type1_no_conflict_when_only_one_dummy() {
	for v in ["a", "b", "c", "d"] {
		let mut g = t1_base();
		if let Some(n) = g.node_mut(v) {
			n.dummy = Some(Dummy::Edge);
		}
		let layering = build_layer_matrix(&g);
		let c = find_type1_conflicts(&g, &layering);
		assert!(!has_conflict(&c, ix(&g, "a"), ix(&g, "d")));
		assert!(!has_conflict(&c, ix(&g, "b"), ix(&g, "c")));
	}
}

#[test]
fn type1_marks_conflict_with_three_dummies() {
	for v in ["a", "b", "c", "d"] {
		let mut g = t1_base();
		for w in ["a", "b", "c", "d"] {
			if w != v
				&& let Some(n) = g.node_mut(w)
			{
				n.dummy = Some(Dummy::Edge);
			}
		}
		let layering = build_layer_matrix(&g);
		let c = find_type1_conflicts(&g, &layering);
		if v == "a" || v == "d" {
			assert!(has_conflict(&c, ix(&g, "a"), ix(&g, "d")), "v={v}");
			assert!(!has_conflict(&c, ix(&g, "b"), ix(&g, "c")), "v={v}");
		} else {
			assert!(!has_conflict(&c, ix(&g, "a"), ix(&g, "d")), "v={v}");
			assert!(has_conflict(&c, ix(&g, "b"), ix(&g, "c")), "v={v}");
		}
	}
}

#[test]
fn type1_no_conflict_when_all_dummies() {
	let mut g = t1_base();
	for v in ["a", "b", "c", "d"] {
		if let Some(n) = g.node_mut(v) {
			n.dummy = Some(Dummy::Edge);
		}
	}
	let layering = build_layer_matrix(&g);
	let c = find_type1_conflicts(&g, &layering);
	assert!(!has_conflict(&c, ix(&g, "a"), ix(&g, "d")));
	assert!(!has_conflict(&c, ix(&g, "b"), ix(&g, "c")));
}

// ---------- findType2Conflicts ------------------------------------------

#[test]
fn type2_favors_border_segments_1() {
	let mut g = t1_base();
	if let Some(n) = g.node_mut("a") {
		n.dummy = Some(Dummy::Edge);
	}
	if let Some(n) = g.node_mut("d") {
		n.dummy = Some(Dummy::Edge);
	}
	if let Some(n) = g.node_mut("b") {
		n.dummy = Some(Dummy::Border);
	}
	if let Some(n) = g.node_mut("c") {
		n.dummy = Some(Dummy::Border);
	}
	let layering = build_layer_matrix(&g);
	let c = find_type2_conflicts(&g, &layering);
	assert!(has_conflict(&c, ix(&g, "a"), ix(&g, "d")));
	assert!(!has_conflict(&c, ix(&g, "b"), ix(&g, "c")));
}

#[test]
fn type2_favors_border_segments_2() {
	let mut g = t1_base();
	if let Some(n) = g.node_mut("b") {
		n.dummy = Some(Dummy::Edge);
	}
	if let Some(n) = g.node_mut("c") {
		n.dummy = Some(Dummy::Edge);
	}
	if let Some(n) = g.node_mut("a") {
		n.dummy = Some(Dummy::Border);
	}
	if let Some(n) = g.node_mut("d") {
		n.dummy = Some(Dummy::Border);
	}
	let layering = build_layer_matrix(&g);
	let c = find_type2_conflicts(&g, &layering);
	assert!(!has_conflict(&c, ix(&g, "a"), ix(&g, "d")));
	assert!(has_conflict(&c, ix(&g, "b"), ix(&g, "c")));
}

// ---------- verticalAlignment -------------------------------------------

#[test]
fn vertical_alignment_self_when_no_adj() {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	g.set_node("b", node(1, 0));
	let layering = build_layer_matrix(&g);
	let conflicts: Conflicts = Conflicts::default();
	let g_ref = &g;
	let (root, align) = vertical_alignment(&layering, &conflicts, |v, out| {
		g_ref.predecessors_into(v, out);
	});
	assert_eq!(root, smap(&g, &[("a", "a"), ("b", "b")]));
	assert_eq!(align, smap(&g, &[("a", "a"), ("b", "b")]));
}

#[test]
fn vertical_alignment_sole_adjacency() {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	g.set_node("b", node(1, 0));
	g.set_edge_default("a", "b");
	let layering = build_layer_matrix(&g);
	let conflicts: Conflicts = Conflicts::default();
	let g_ref = &g;
	let (root, align) = vertical_alignment(&layering, &conflicts, |v, out| {
		g_ref.predecessors_into(v, out);
	});
	assert_eq!(root, smap(&g, &[("a", "a"), ("b", "a")]));
	assert_eq!(align, smap(&g, &[("a", "b"), ("b", "a")]));
}

#[test]
fn vertical_alignment_left_median() {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	g.set_node("b", node(0, 1));
	g.set_node("c", node(1, 0));
	g.set_edge_default("a", "c");
	g.set_edge_default("b", "c");
	let layering = build_layer_matrix(&g);
	let conflicts: Conflicts = Conflicts::default();
	let g_ref = &g;
	let (root, align) = vertical_alignment(&layering, &conflicts, |v, out| {
		g_ref.predecessors_into(v, out);
	});
	assert_eq!(root, smap(&g, &[("a", "a"), ("b", "b"), ("c", "a")]));
	assert_eq!(align, smap(&g, &[("a", "c"), ("b", "b"), ("c", "a")]));
}

#[test]
fn vertical_alignment_right_median_when_left_blocked() {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	g.set_node("b", node(0, 1));
	g.set_node("c", node(1, 0));
	g.set_edge_default("a", "c");
	g.set_edge_default("b", "c");
	let layering = build_layer_matrix(&g);
	let mut conflicts: Conflicts = Conflicts::default();
	add_conflict(&mut conflicts, ix(&g, "a"), ix(&g, "c"));
	let g_ref = &g;
	let (root, align) = vertical_alignment(&layering, &conflicts, |v, out| {
		g_ref.predecessors_into(v, out);
	});
	assert_eq!(root, smap(&g, &[("a", "a"), ("b", "b"), ("c", "b")]));
	assert_eq!(align, smap(&g, &[("a", "a"), ("b", "c"), ("c", "b")]));
}

#[test]
fn vertical_alignment_single_median_for_odd_adjacencies() {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	g.set_node("b", node(0, 1));
	g.set_node("c", node(0, 2));
	g.set_node("d", node(1, 0));
	g.set_edge_default("a", "d");
	g.set_edge_default("b", "d");
	g.set_edge_default("c", "d");
	let layering = build_layer_matrix(&g);
	let conflicts: Conflicts = Conflicts::default();
	let g_ref = &g;
	let (root, align) = vertical_alignment(&layering, &conflicts, |v, out| {
		g_ref.predecessors_into(v, out);
	});
	assert_eq!(
		root,
		smap(&g, &[("a", "a"), ("b", "b"), ("c", "c"), ("d", "b")])
	);
	assert_eq!(
		align,
		smap(&g, &[("a", "a"), ("b", "d"), ("c", "c"), ("d", "b")])
	);
}

// ---------- horizontalCompaction ----------------------------------------

#[test]
fn hc_single_node_at_origin() {
	let mut g = mk();
	g.set_node("a", node(0, 0));
	let root = smap(&g, &[("a", "a")]);
	let align = smap(&g, &[("a", "a")]);
	let xs = horizontal_compaction(
		&g,
		&build_layer_matrix(&g),
		&root,
		&align,
		false,
	);
	assert_eq!(xs.get_copied(ix(&g, "a")), Some(0.0));
}

#[test]
fn hc_separates_adjacent_nodes_by_nodesep() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.nodesep = Some(100.0);
	}
	g.set_node("a", n_w(0, 0, 100.0));
	g.set_node("b", n_w(0, 1, 200.0));
	let root = smap(&g, &[("a", "a"), ("b", "b")]);
	let align = smap(&g, &[("a", "a"), ("b", "b")]);
	let xs = horizontal_compaction(
		&g,
		&build_layer_matrix(&g),
		&root,
		&align,
		false,
	);
	assert_eq!(xs.get_copied(ix(&g, "a")), Some(0.0));
	assert_eq!(xs.get_copied(ix(&g, "b")), Some(50.0 + 100.0 + 100.0));
}

#[test]
fn hc_separates_adjacent_edges_by_edgesep() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.edgesep = Some(20.0);
	}
	let mut a = n_w(0, 0, 100.0);
	a.dummy = Some(Dummy::Edge);
	let mut b = n_w(0, 1, 200.0);
	b.dummy = Some(Dummy::Edge);
	g.set_node("a", a);
	g.set_node("b", b);
	let root = smap(&g, &[("a", "a"), ("b", "b")]);
	let align = smap(&g, &[("a", "a"), ("b", "b")]);
	let xs = horizontal_compaction(
		&g,
		&build_layer_matrix(&g),
		&root,
		&align,
		false,
	);
	assert_eq!(xs.get_copied(ix(&g, "a")), Some(0.0));
	assert_eq!(xs.get_copied(ix(&g, "b")), Some(50.0 + 20.0 + 100.0));
}

#[test]
fn hc_aligns_centers_in_same_block() {
	let mut g = mk();
	g.set_node("a", n_w(0, 0, 100.0));
	g.set_node("b", n_w(1, 0, 200.0));
	let root = smap(&g, &[("a", "a"), ("b", "a")]);
	let align = smap(&g, &[("a", "b"), ("b", "a")]);
	let xs = horizontal_compaction(
		&g,
		&build_layer_matrix(&g),
		&root,
		&align,
		false,
	);
	assert_eq!(xs.get_copied(ix(&g, "a")), Some(0.0));
	assert_eq!(xs.get_copied(ix(&g, "b")), Some(0.0));
}

#[test]
fn hc_handles_labelpos_l() {
	let mut g = mk();
	if let Some(gl) = g.graph_mut() {
		gl.edgesep = Some(50.0);
	}
	let mut a = n_w(0, 0, 100.0);
	a.dummy = Some(Dummy::Edge);
	let mut b = n_w(0, 1, 200.0);
	b.dummy = Some(Dummy::EdgeLabel);
	b.labelpos = Some(LabelPos::L);
	let mut c = n_w(0, 2, 300.0);
	c.dummy = Some(Dummy::Edge);
	g.set_node("a", a);
	g.set_node("b", b);
	g.set_node("c", c);
	let root = smap(&g, &[("a", "a"), ("b", "b"), ("c", "c")]);
	let align = smap(&g, &[("a", "a"), ("b", "b"), ("c", "c")]);
	let xs = horizontal_compaction(
		&g,
		&build_layer_matrix(&g),
		&root,
		&align,
		false,
	);
	let xa = xs[ix(&g, "a")];
	let xb = xs[ix(&g, "b")];
	let xc = xs[ix(&g, "c")];
	assert_eq!(xa, 0.0);
	assert!((xb - (xa + 50.0 + 50.0 + 200.0)).abs() < 1e-9, "b={xb}");
	assert!((xc - (xb + 0.0 + 50.0 + 150.0)).abs() < 1e-9, "c={xc}");
}

// ---------- alignCoordinates --------------------------------------------

#[test]
fn align_coords_single_node() {
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert("ul".into(), tmap(&[("a", 50.0)]));
	xss.insert("ur".into(), tmap(&[("a", 100.0)]));
	xss.insert("dl".into(), tmap(&[("a", 50.0)]));
	xss.insert("dr".into(), tmap(&[("a", 200.0)]));
	let align_to = xss["ul"].clone();
	align_coordinates(&mut xss, &align_to);
	assert_eq!(xss["ul"], tmap(&[("a", 50.0)]));
	assert_eq!(xss["ur"], tmap(&[("a", 50.0)]));
	assert_eq!(xss["dl"], tmap(&[("a", 50.0)]));
	assert_eq!(xss["dr"], tmap(&[("a", 50.0)]));
}

#[test]
fn align_coords_multi_node() {
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert("ul".into(), tmap(&[("a", 50.0), ("b", 1000.0)]));
	xss.insert("ur".into(), tmap(&[("a", 100.0), ("b", 900.0)]));
	xss.insert("dl".into(), tmap(&[("a", 150.0), ("b", 800.0)]));
	xss.insert("dr".into(), tmap(&[("a", 200.0), ("b", 700.0)]));
	let align_to = xss["ul"].clone();
	align_coordinates(&mut xss, &align_to);
	assert_eq!(xss["ul"], tmap(&[("a", 50.0), ("b", 1000.0)]));
	assert_eq!(xss["ur"], tmap(&[("a", 200.0), ("b", 1000.0)]));
	assert_eq!(xss["dl"], tmap(&[("a", 50.0), ("b", 700.0)]));
	assert_eq!(xss["dr"], tmap(&[("a", 500.0), ("b", 1000.0)]));
}

// ---------- findSmallestWidthAlignment ---------------------------------

#[test]
fn smallest_width_basic() {
	let mut g = mk();
	g.set_node("a", n_w(0, 0, 50.0));
	g.set_node("b", n_w(0, 1, 50.0));
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert("ul".into(), pmap(&g, &[("a", 0.0), ("b", 1000.0)]));
	xss.insert("ur".into(), pmap(&g, &[("a", -5.0), ("b", 1000.0)]));
	xss.insert("dl".into(), pmap(&g, &[("a", 5.0), ("b", 2000.0)]));
	xss.insert("dr".into(), pmap(&g, &[("a", 0.0), ("b", 200.0)]));
	let r = find_smallest_width_alignment(&g, &xss);
	assert_eq!(r, xss["dr"]);
}

#[test]
fn smallest_width_uses_node_width() {
	let mut g = mk();
	g.set_node("a", n_w(0, 0, 50.0));
	g.set_node("b", n_w(0, 1, 50.0));
	g.set_node("c", n_w(0, 2, 200.0));
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert(
		"ul".into(),
		pmap(&g, &[("a", 0.0), ("b", 100.0), ("c", 75.0)]),
	);
	xss.insert(
		"ur".into(),
		pmap(&g, &[("a", 0.0), ("b", 100.0), ("c", 80.0)]),
	);
	xss.insert(
		"dl".into(),
		pmap(&g, &[("a", 0.0), ("b", 100.0), ("c", 85.0)]),
	);
	xss.insert(
		"dr".into(),
		pmap(&g, &[("a", 0.0), ("b", 100.0), ("c", 90.0)]),
	);
	let r = find_smallest_width_alignment(&g, &xss);
	assert_eq!(r, xss["ul"]);
}

// ---------- balance -----------------------------------------------------

#[test]
fn balance_single_shared_median() {
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert("ul".into(), tmap(&[("a", 0.0)]));
	xss.insert("ur".into(), tmap(&[("a", 100.0)]));
	xss.insert("dl".into(), tmap(&[("a", 100.0)]));
	xss.insert("dr".into(), tmap(&[("a", 200.0)]));
	assert_eq!(balance(&xss, None), tmap(&[("a", 100.0)]));
}

#[test]
fn balance_single_avg_of_different() {
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert("ul".into(), tmap(&[("a", 0.0)]));
	xss.insert("ur".into(), tmap(&[("a", 75.0)]));
	xss.insert("dl".into(), tmap(&[("a", 125.0)]));
	xss.insert("dr".into(), tmap(&[("a", 200.0)]));
	assert_eq!(balance(&xss, None), tmap(&[("a", 100.0)]));
}

#[test]
fn balance_multi_node() {
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	xss.insert("ul".into(), tmap(&[("a", 0.0), ("b", 50.0)]));
	xss.insert("ur".into(), tmap(&[("a", 75.0), ("b", 0.0)]));
	xss.insert("dl".into(), tmap(&[("a", 125.0), ("b", 60.0)]));
	xss.insert("dr".into(), tmap(&[("a", 200.0), ("b", 75.0)]));
	assert_eq!(balance(&xss, None), tmap(&[("a", 100.0), ("b", 55.0)]));
}

// ---------- positionX ---------------------------------------------------

#[test]
fn positionx_single_node_at_origin() {
	let mut g = mk();
	g.set_node("a", n_w(0, 0, 100.0));
	let pos = position_x(&g);
	assert_eq!(pos.get_copied(ix(&g, "a")), Some(0.0));
}

#[test]
fn positionx_single_block_at_origin() {
	let mut g = mk();
	g.set_node("a", n_w(0, 0, 100.0));
	g.set_node("b", n_w(1, 0, 100.0));
	g.set_edge_default("a", "b");
	let pos = position_x(&g);
	assert_eq!(pos.get_copied(ix(&g, "a")), Some(0.0));
	assert_eq!(pos.get_copied(ix(&g, "b")), Some(0.0));
}

#[test]
fn positionx_block_with_different_sizes() {
	let mut g = mk();
	g.set_node("a", n_w(0, 0, 40.0));
	g.set_node("b", n_w(1, 0, 500.0));
	g.set_node("c", n_w(2, 0, 20.0));
	g.set_path(&["a", "b", "c"]);
	let pos = position_x(&g);
	assert_eq!(pos.get_copied(ix(&g, "a")), Some(0.0));
	assert_eq!(pos.get_copied(ix(&g, "b")), Some(0.0));
	assert_eq!(pos.get_copied(ix(&g, "c")), Some(0.0));
}
