//! Port of test/rank/network-simplex-test.ts.

use dagre::{
	graph::{Edge, Graph, GraphOpts},
	rank::{
		feasible_tree::{Tree, TreeEdge, TreeNode},
		network_simplex,
		util_rank::longest_path,
	},
	types::{EdgeLabel, GraphLabel, NodeLabel},
	util::normalize_ranks,
};

fn mk_g() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
		Graph::with_opts(GraphOpts::directed().multigraph());
	g.set_default_edge_label(|_| EdgeLabel {
		minlen: 1,
		weight: 1.0,
		..Default::default()
	});
	g
}

fn mk_t() -> Tree {
	let g: Tree = Graph::with_opts(GraphOpts::undirected());
	g
}

fn ns(g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	network_simplex::run(g);
	normalize_ranks(g);
}

fn gansner_graph() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g = mk_g();
	g.set_path(&["a", "b", "c", "d", "h"]);
	g.set_path(&["a", "e", "g", "h"]);
	g.set_path(&["a", "f", "g"]);
	g
}

fn gansner_tree() -> Tree {
	let mut t = mk_t();
	// Setting up the same tree as gansnerTree in JS.
	// Undirected edges: a-b, b-c, c-d, d-h, h-g, g-e, g-f.
	for (v, w) in [
		("a", "b"),
		("b", "c"),
		("c", "d"),
		("d", "h"),
		("h", "g"),
		("g", "e"),
		("g", "f"),
	] {
		if !t.has_node(v) {
			t.set_node(v.to_string(), TreeNode::default());
		}
		if !t.has_node(w) {
			t.set_node(w.to_string(), TreeNode::default());
		}
		t.set_edge(v.to_string(), w.to_string(), TreeEdge::default());
	}
	t
}

fn undir(e: &Edge) -> (String, String) {
	if e.v < e.w {
		(e.v.clone(), e.w.clone())
	} else {
		(e.w.clone(), e.v.clone())
	}
}

// ---------- main entry --------------------------------------------------

#[test]
fn ranks_single_node() {
	let mut g = mk_g();
	g.set_node("a", NodeLabel::default());
	ns(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
}

#[test]
fn ranks_two_node_connected_graph() {
	let mut g = mk_g();
	g.set_edge_default("a", "b");
	ns(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
}

#[test]
fn ranks_a_diamond() {
	let mut g = mk_g();
	g.set_path(&["a", "b", "d"]);
	g.set_path(&["a", "c", "d"]);
	ns(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
	assert_eq!(g.node("c").unwrap().rank, Some(1));
	assert_eq!(g.node("d").unwrap().rank, Some(2));
}

#[test]
fn uses_minlen_on_edge() {
	let mut g = mk_g();
	g.set_path(&["a", "b", "d"]);
	g.set_edge_default("a", "c");
	g.set_edge(
		"c",
		"d",
		EdgeLabel {
			minlen: 2,
			weight: 1.0,
			..Default::default()
		},
	);
	ns(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(2));
	assert_eq!(g.node("c").unwrap().rank, Some(1));
	assert_eq!(g.node("d").unwrap().rank, Some(3));
}

#[test]
fn ranks_the_gansner_graph() {
	let mut g = gansner_graph();
	ns(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
	assert_eq!(g.node("c").unwrap().rank, Some(2));
	assert_eq!(g.node("d").unwrap().rank, Some(3));
	assert_eq!(g.node("h").unwrap().rank, Some(4));
	assert_eq!(g.node("e").unwrap().rank, Some(1));
	assert_eq!(g.node("f").unwrap().rank, Some(1));
	assert_eq!(g.node("g").unwrap().rank, Some(2));
}

#[test]
fn handles_multi_edges() {
	let mut g = mk_g();
	g.set_path(&["a", "b", "c", "d"]);
	g.set_edge(
		"a",
		"e",
		EdgeLabel {
			weight: 2.0,
			minlen: 1,
			..Default::default()
		},
	);
	g.set_edge_default("e", "d");
	g.set_edge_named(
		"b",
		"c",
		EdgeLabel {
			weight: 1.0,
			minlen: 2,
			..Default::default()
		},
		Some("multi".into()),
	);
	ns(&mut g);
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
	assert_eq!(g.node("c").unwrap().rank, Some(3));
	assert_eq!(g.node("d").unwrap().rank, Some(4));
	assert_eq!(g.node("e").unwrap().rank, Some(1));
}

// ---------- leaveEdge ---------------------------------------------------

#[test]
fn leave_edge_returns_none_when_all_cutvalues_nonneg() {
	let mut t = mk_t();
	t.set_node("a", TreeNode::default());
	t.set_node("b", TreeNode::default());
	t.set_node("c", TreeNode::default());
	t.set_edge(
		"a",
		"b",
		TreeEdge {
			cutvalue: Some(1.0),
		},
	);
	t.set_edge(
		"b",
		"c",
		TreeEdge {
			cutvalue: Some(1.0),
		},
	);
	assert!(network_simplex::leave_edge(&t).is_none());
}

#[test]
fn leave_edge_returns_negative_cutvalue_edge() {
	let mut t = mk_t();
	t.set_node("a", TreeNode::default());
	t.set_node("b", TreeNode::default());
	t.set_node("c", TreeNode::default());
	t.set_edge(
		"a",
		"b",
		TreeEdge {
			cutvalue: Some(1.0),
		},
	);
	t.set_edge(
		"b",
		"c",
		TreeEdge {
			cutvalue: Some(-1.0),
		},
	);
	let e = network_simplex::leave_edge(&t).unwrap();
	assert_eq!(undir(&e), ("b".to_string(), "c".to_string()));
}

// ---------- initLowLimValues --------------------------------------------

#[test]
fn init_low_lim_assigns_low_lim_parent() {
	let mut g: Tree = mk_t();
	for v in ["a", "b", "c", "d", "e"] {
		g.set_node(v.to_string(), TreeNode::default());
	}
	// The JS test setPath ["a", "b", "a", "c", "d", "c", "e"] on an undirected
	// graph collapses to edges {a,b}, {a,c}, {c,d}, {c,e}.
	for (v, w) in [("a", "b"), ("a", "c"), ("c", "d"), ("c", "e")] {
		g.set_edge(v.to_string(), w.to_string(), TreeEdge::default());
	}

	network_simplex::init_low_lim(&mut g, Some("a".into()));

	let lim_a = g.node("a").unwrap().lim.unwrap();
	let lim_b = g.node("b").unwrap().lim.unwrap();
	let lim_c = g.node("c").unwrap().lim.unwrap();
	let lim_d = g.node("d").unwrap().lim.unwrap();
	let lim_e = g.node("e").unwrap().lim.unwrap();

	let mut all = vec![lim_a, lim_b, lim_c, lim_d, lim_e];
	all.sort_unstable();
	assert_eq!(all, vec![1, 2, 3, 4, 5]);

	assert_eq!(g.node("a").unwrap().low, Some(1));
	assert_eq!(g.node("a").unwrap().lim, Some(5));

	assert_eq!(g.node("b").unwrap().parent.as_deref(), Some("a"));
	assert!(lim_b < lim_a);

	assert_eq!(g.node("c").unwrap().parent.as_deref(), Some("a"));
	assert!(lim_c < lim_a);
	assert_ne!(lim_b, lim_c);

	assert_eq!(g.node("d").unwrap().parent.as_deref(), Some("c"));
	assert!(lim_d < lim_c);

	assert_eq!(g.node("e").unwrap().parent.as_deref(), Some("c"));
	assert!(lim_e < lim_c);
	assert_ne!(lim_d, lim_e);
}

// ---------- exchangeEdges -----------------------------------------------

#[test]
fn exchange_edges_updates_cutvalues_and_lims() {
	let mut g = gansner_graph();
	let mut t = gansner_tree();
	longest_path(&mut g);
	network_simplex::init_low_lim(&mut t, None);

	network_simplex::exchange_edges(
		&mut t,
		&mut g,
		&Edge::new("g", "h"),
		&Edge::new("a", "e"),
	);

	assert_eq!(t.edge("a", "b").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("b", "c").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("c", "d").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("d", "h").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("a", "e").unwrap().cutvalue, Some(1.0));
	assert_eq!(t.edge("e", "g").unwrap().cutvalue, Some(1.0));
	assert_eq!(t.edge("g", "f").unwrap().cutvalue, Some(0.0));

	let mut lims: Vec<i32> = t
		.nodes()
		.iter()
		.map(|v| t.node(v).unwrap().lim.unwrap())
		.collect();
	lims.sort();
	assert_eq!(lims, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn exchange_edges_updates_ranks() {
	let mut g = gansner_graph();
	let mut t = gansner_tree();
	longest_path(&mut g);
	network_simplex::init_low_lim(&mut t, None);

	network_simplex::exchange_edges(
		&mut t,
		&mut g,
		&Edge::new("g", "h"),
		&Edge::new("a", "e"),
	);
	normalize_ranks(&mut g);

	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
	assert_eq!(g.node("c").unwrap().rank, Some(2));
	assert_eq!(g.node("d").unwrap().rank, Some(3));
	assert_eq!(g.node("e").unwrap().rank, Some(1));
	assert_eq!(g.node("f").unwrap().rank, Some(1));
	assert_eq!(g.node("g").unwrap().rank, Some(2));
	assert_eq!(g.node("h").unwrap().rank, Some(4));
}

// ---------- calcCutValue ------------------------------------------------
//
// Helper layout: parent (`p`), child (`c`), grandchild (`gc`), other (`o`).

fn calc_cut_value_setup(
	g_edges: &[(&str, &str, f64)],
	t_edges: &[(&str, &str, Option<f64>)],
) -> (Graph<GraphLabel, NodeLabel, EdgeLabel>, Tree) {
	let mut g = mk_g();
	for (v, w, weight) in g_edges {
		g.set_edge(
			v.to_string(),
			w.to_string(),
			EdgeLabel {
				weight: *weight,
				minlen: 1,
				..Default::default()
			},
		);
	}
	let mut t = mk_t();
	for (v, w, cv) in t_edges {
		if !t.has_node(v) {
			t.set_node(v.to_string(), TreeNode::default());
		}
		if !t.has_node(w) {
			t.set_node(w.to_string(), TreeNode::default());
		}
		t.set_edge(v.to_string(), w.to_string(), TreeEdge { cutvalue: *cv });
	}
	network_simplex::init_low_lim(&mut t, Some("p".into()));
	(g, t)
}

#[test]
fn calc_cut_two_node_c_to_p() {
	let (g, t) = calc_cut_value_setup(&[("c", "p", 1.0)], &[("p", "c", None)]);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), 1.0);
}

#[test]
fn calc_cut_two_node_p_to_c() {
	let (g, t) = calc_cut_value_setup(&[("p", "c", 1.0)], &[("p", "c", None)]);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), 1.0);
}

#[test]
fn calc_cut_3_node_gc_c_p_pointing_to_p() {
	let (g, t) = calc_cut_value_setup(
		&[("gc", "c", 1.0), ("c", "p", 1.0)],
		&[("gc", "c", Some(3.0)), ("p", "c", None)],
	);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), 3.0);
}

#[test]
fn calc_cut_3_node_gc_in_p_in() {
	let (g, t) = calc_cut_value_setup(
		&[("p", "c", 1.0), ("gc", "c", 1.0)],
		&[("gc", "c", Some(3.0)), ("p", "c", None)],
	);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), -1.0);
}

#[test]
fn calc_cut_3_node_c_to_both() {
	let (g, t) = calc_cut_value_setup(
		&[("c", "p", 1.0), ("c", "gc", 1.0)],
		&[("gc", "c", Some(3.0)), ("p", "c", None)],
	);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), -1.0);
}

#[test]
fn calc_cut_3_node_p_to_c_to_gc() {
	let (g, t) = calc_cut_value_setup(
		&[("p", "c", 1.0), ("c", "gc", 1.0)],
		&[("gc", "c", Some(3.0)), ("p", "c", None)],
	);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), 3.0);
}

#[test]
fn calc_cut_4_node_gc_c_p_o_with_o_to_c() {
	let (g, t) = calc_cut_value_setup(
		&[
			("o", "c", 7.0),
			("gc", "c", 1.0),
			("c", "p", 1.0),
			("p", "o", 1.0),
		],
		&[("gc", "c", Some(3.0)), ("c", "p", None), ("p", "o", None)],
	);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), -4.0);
}

#[test]
fn calc_cut_4_node_gc_c_p_o_with_c_to_o() {
	let (g, t) = calc_cut_value_setup(
		&[
			("c", "o", 7.0),
			("gc", "c", 1.0),
			("c", "p", 1.0),
			("p", "o", 1.0),
		],
		&[("gc", "c", Some(3.0)), ("c", "p", None), ("p", "o", None)],
	);
	assert_eq!(network_simplex::calc_cut_value(&t, &g, "c", "p"), 10.0);
}

// ---------- initCutValues -----------------------------------------------

#[test]
fn init_cut_values_works_for_gansner_graph() {
	let g = gansner_graph();
	let mut t = gansner_tree();
	network_simplex::init_low_lim(&mut t, None);
	network_simplex::init_cut_values(&mut t, &g);
	assert_eq!(t.edge("a", "b").unwrap().cutvalue, Some(3.0));
	assert_eq!(t.edge("b", "c").unwrap().cutvalue, Some(3.0));
	assert_eq!(t.edge("c", "d").unwrap().cutvalue, Some(3.0));
	assert_eq!(t.edge("d", "h").unwrap().cutvalue, Some(3.0));
	assert_eq!(t.edge("g", "h").unwrap().cutvalue, Some(-1.0));
	assert_eq!(t.edge("e", "g").unwrap().cutvalue, Some(0.0));
	assert_eq!(t.edge("f", "g").unwrap().cutvalue, Some(0.0));
}

#[test]
fn init_cut_values_works_for_updated_gansner_graph() {
	let g = gansner_graph();
	let mut t = gansner_tree();
	t.remove_edge("g", "h");
	if !t.has_node("a") {
		t.set_node("a", TreeNode::default());
	}
	if !t.has_node("e") {
		t.set_node("e", TreeNode::default());
	}
	t.set_edge("a", "e", TreeEdge::default());
	network_simplex::init_low_lim(&mut t, None);
	network_simplex::init_cut_values(&mut t, &g);
	assert_eq!(t.edge("a", "b").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("b", "c").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("c", "d").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("d", "h").unwrap().cutvalue, Some(2.0));
	assert_eq!(t.edge("a", "e").unwrap().cutvalue, Some(1.0));
	assert_eq!(t.edge("e", "g").unwrap().cutvalue, Some(1.0));
	assert_eq!(t.edge("f", "g").unwrap().cutvalue, Some(0.0));
}
