//! Port of test/acyclic-test.ts.

use std::string::ToString;

use dagre::{
	acyclic,
	graph::{Edge, Graph, GraphOpts, NodeId, alg},
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

fn mk_graph(
	acyclicer: Option<&str>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
		Graph::with_opts(GraphOpts::directed().multigraph());
	g.set_default_edge_label(|_| EdgeLabel {
		minlen: 1,
		weight: 1.0,
		..Default::default()
	});
	g.set_graph(GraphLabel {
		acyclicer: acyclicer.map(ToString::to_string),
		..Default::default()
	});
	g
}

fn strip(e: &Edge) -> (NodeId, NodeId) {
	(e.v.clone(), e.w.clone())
}

fn sort_edges(edges: &mut [Edge]) {
	edges.sort_by(|a, b| match (&a.name, &b.name) {
		(Some(an), Some(bn)) => an.cmp(bn),
		_ => match a.v.cmp(&b.v) {
			std::cmp::Ordering::Equal => a.w.cmp(&b.w),
			o => o,
		},
	});
}

fn run_for_each_acyclicer<F: Fn(Option<&str>)>(test: F) {
	for ac in [Some("greedy"), Some("dfs"), Some("unknown")] {
		test(ac);
	}
}

#[test]
fn run_does_not_change_an_already_acyclic_graph() {
	run_for_each_acyclicer(|ac| {
		let mut g = mk_graph(ac);
		// a -> b -> d, a -> c -> d
		g.set_edge_default("a", "b");
		g.set_edge_default("b", "d");
		g.set_edge_default("a", "c");
		g.set_edge_default("c", "d");
		acyclic::run(&mut g);
		let mut edges: Vec<Edge> = g.edges();
		sort_edges(&mut edges);
		let pairs: Vec<(NodeId, NodeId)> = edges.iter().map(strip).collect();
		let expected: Vec<(NodeId, NodeId)> = vec![
			("a".into(), "b".into()),
			("a".into(), "c".into()),
			("b".into(), "d".into()),
			("c".into(), "d".into()),
		];
		assert_eq!(pairs, expected, "acyclicer={ac:?}");
	});
}

#[test]
fn run_breaks_cycles_in_the_input_graph() {
	run_for_each_acyclicer(|ac| {
		let mut g = mk_graph(ac);
		g.set_path(&["a", "b", "c", "d", "a"]);
		acyclic::run(&mut g);
		let cycles = alg::find_cycles(&g);
		assert!(cycles.is_empty(), "still cyclic ({ac:?}): {cycles:?}");
	});
}

#[test]
fn run_creates_multi_edge_where_necessary() {
	run_for_each_acyclicer(|ac| {
		let mut g = mk_graph(ac);
		g.set_path(&["a", "b", "a"]);
		acyclic::run(&mut g);
		let cycles = alg::find_cycles(&g);
		assert!(cycles.is_empty(), "still cyclic ({ac:?})");
		assert_eq!(g.edge_count(), 2);
		let ab = g
			.out_edges_to("a", "b")
			.unwrap_or_default();
		let ba = g
			.out_edges_to("b", "a")
			.unwrap_or_default();
		assert!(
			ab.len() == 2 || ba.len() == 2,
			"expected two edges in same direction"
		);
	});
}

#[test]
fn undo_leaves_acyclic_input_unchanged() {
	run_for_each_acyclicer(|ac| {
		let mut g = mk_graph(ac);
		g.set_edge(
			"a",
			"b",
			EdgeLabel {
				minlen: 2,
				weight: 3.0,
				..Default::default()
			},
		);
		acyclic::run(&mut g);
		acyclic::undo(&mut g);
		let e = g.edge("a", "b").unwrap();
		assert_eq!(e.minlen, 2);
		assert_eq!(e.weight, 3.0);
		assert_eq!(g.edge_count(), 1);
	});
}

#[test]
fn undo_restores_reversed_edges() {
	run_for_each_acyclicer(|ac| {
		let mut g = mk_graph(ac);
		g.set_edge(
			"a",
			"b",
			EdgeLabel {
				minlen: 2,
				weight: 3.0,
				..Default::default()
			},
		);
		g.set_edge(
			"b",
			"a",
			EdgeLabel {
				minlen: 3,
				weight: 4.0,
				..Default::default()
			},
		);
		acyclic::run(&mut g);
		acyclic::undo(&mut g);
		let ab = g.edge("a", "b").unwrap();
		assert_eq!((ab.minlen, ab.weight), (2, 3.0));
		let ba = g.edge("b", "a").unwrap();
		assert_eq!((ba.minlen, ba.weight), (3, 4.0));
		assert_eq!(g.edge_count(), 2);
	});
}

#[test]
fn greedy_breaks_at_low_weight_edges() {
	let mut g = mk_graph(Some("greedy"));
	g.set_default_edge_label(|_| EdgeLabel {
		minlen: 1,
		weight: 2.0,
		..Default::default()
	});
	g.set_path(&["a", "b", "c", "d", "a"]);
	g.set_edge(
		"c",
		"d",
		EdgeLabel {
			weight: 1.0,
			minlen: 1,
			..Default::default()
		},
	);
	acyclic::run(&mut g);
	let cycles = alg::find_cycles(&g);
	assert!(cycles.is_empty());
	assert!(!g.has_edge("c", "d"), "greedy should reverse c->d");
}
