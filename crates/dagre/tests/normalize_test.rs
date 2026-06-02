//! Port of test/normalize-test.ts.

use dagre::{
	graph::{Graph, GraphOpts},
	normalize,
	types::{Dummy, EdgeLabel, GraphLabel, NodeLabel, Point},
};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::with_opts(
		GraphOpts::directed()
			.multigraph()
			.compound(),
	);
	g.set_graph(GraphLabel::default());
	g
}

#[test]
fn run_does_not_change_a_short_edge() {
	let mut g = mk();
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
	g.set_edge("a", "b", EdgeLabel::default());
	normalize::run(&mut g);
	let es = g.edges();
	assert_eq!(es.len(), 1);
	assert_eq!((es[0].v.as_str(), es[0].w.as_str()), ("a", "b"));
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(1));
}

#[test]
fn run_splits_two_layer_edge_into_two_segments() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge("a", "b", EdgeLabel::default());
	normalize::run(&mut g);
	let succ = g.successors("a").unwrap();
	assert_eq!(succ.len(), 1);
	let dummy = &succ[0];
	let dn = g.node(dummy).unwrap();
	assert_eq!(dn.dummy, Some(Dummy::Edge));
	assert_eq!(dn.rank, Some(1));
	assert_eq!(g.successors(dummy).unwrap(), vec!["b".to_string()]);
	let chains = g
		.graph()
		.unwrap()
		.dummy_chains
		.clone()
		.unwrap_or_default();
	assert_eq!(chains.len(), 1);
	assert_eq!(&chains[0], dummy);
}

#[test]
fn run_assigns_zero_dims_to_dummy_nodes_by_default() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			width: 10.0,
			height: 10.0,
			..Default::default()
		},
	);
	normalize::run(&mut g);
	let dummy = g.successors("a").unwrap()[0].clone();
	let dn = g.node(&dummy).unwrap();
	assert_eq!(dn.width, 0.0);
	assert_eq!(dn.height, 0.0);
}

#[test]
fn run_assigns_dims_from_edge_for_node_on_label_rank() {
	let mut g = mk();
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
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			width: 20.0,
			height: 10.0,
			label_rank: Some(2),
			..Default::default()
		},
	);
	normalize::run(&mut g);
	let first = g.successors("a").unwrap()[0].clone();
	let label_v = g.successors(&first).unwrap()[0].clone();
	let label_node = g.node(&label_v).unwrap();
	assert_eq!(label_node.width, 20.0);
	assert_eq!(label_node.height, 10.0);
}

#[test]
fn run_preserves_edge_weight() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			weight: 2.0,
			..Default::default()
		},
	);
	normalize::run(&mut g);
	let succ = g.successors("a").unwrap();
	assert_eq!(succ.len(), 1);
	let e = g.edge("a", &succ[0]).unwrap();
	assert_eq!(e.weight, 2.0);
}

#[test]
fn undo_reverses_run() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge("a", "b", EdgeLabel::default());
	normalize::run(&mut g);
	normalize::undo(&mut g);
	let es = g.edges();
	assert_eq!(es.len(), 1);
	assert_eq!((es[0].v.as_str(), es[0].w.as_str()), ("a", "b"));
	assert_eq!(g.node("a").unwrap().rank, Some(0));
	assert_eq!(g.node("b").unwrap().rank, Some(2));
}

#[test]
fn undo_collects_assigned_coordinates_into_points() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge("a", "b", EdgeLabel::default());
	normalize::run(&mut g);
	let dummy = g.neighbors("a").unwrap()[0].clone();
	if let Some(n) = g.node_mut(&dummy) {
		n.x = Some(5.0);
		n.y = Some(10.0);
	}
	normalize::undo(&mut g);
	let pts = g
		.edge("a", "b")
		.unwrap()
		.points
		.clone()
		.unwrap_or_default();
	assert_eq!(pts, vec![Point { x: 5.0, y: 10.0 }]);
}

#[test]
fn undo_merges_coordinates_along_long_edge() {
	let mut g = mk();
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
	g.set_edge("a", "b", EdgeLabel::default());
	normalize::run(&mut g);
	// Three dummy nodes: a -> d1 -> d2 -> d3 -> b
	let d1 = g.successors("a").unwrap()[0].clone();
	let d2 = g.successors(&d1).unwrap()[0].clone();
	let d3 = g.successors(&d2).unwrap()[0].clone();
	for (v, x, y) in [(&d1, 5.0, 10.0), (&d2, 20.0, 25.0), (&d3, 100.0, 200.0)]
	{
		if let Some(n) = g.node_mut(v) {
			n.x = Some(x);
			n.y = Some(y);
		}
	}
	normalize::undo(&mut g);
	let pts = g
		.edge("a", "b")
		.unwrap()
		.points
		.clone()
		.unwrap_or_default();
	assert_eq!(
		pts,
		vec![
			Point { x: 5.0, y: 10.0 },
			Point { x: 20.0, y: 25.0 },
			Point { x: 100.0, y: 200.0 },
		]
	);
}

#[test]
fn undo_sets_coords_and_dims_when_short_edge_has_label() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge(
		"a",
		"b",
		EdgeLabel {
			width: 10.0,
			height: 20.0,
			label_rank: Some(1),
			..Default::default()
		},
	);
	normalize::run(&mut g);
	let label_v = g.successors("a").unwrap()[0].clone();
	if let Some(n) = g.node_mut(&label_v) {
		n.x = Some(50.0);
		n.y = Some(60.0);
		n.width = 20.0;
		n.height = 10.0;
	}
	normalize::undo(&mut g);
	let e = g.edge("a", "b").unwrap();
	assert_eq!(
		(e.x, e.y, e.width, e.height),
		(Some(50.0), Some(60.0), 20.0, 10.0)
	);
}

#[test]
fn undo_restores_multi_edges() {
	let mut g = mk();
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
			rank: Some(2),
			..Default::default()
		},
	);
	g.set_edge_named("a", "b", EdgeLabel::default(), Some("bar".into()));
	g.set_edge_named("a", "b", EdgeLabel::default(), Some("foo".into()));
	normalize::run(&mut g);

	let mut out_edges = g.out_edges("a").unwrap();
	out_edges.sort_by(|a, b| a.name.cmp(&b.name));
	assert_eq!(out_edges.len(), 2);

	let bar_dummy = out_edges[0].w.clone();
	let foo_dummy = out_edges[1].w.clone();
	if let Some(n) = g.node_mut(&bar_dummy) {
		n.x = Some(5.0);
		n.y = Some(10.0);
	}
	if let Some(n) = g.node_mut(&foo_dummy) {
		n.x = Some(15.0);
		n.y = Some(20.0);
	}
	normalize::undo(&mut g);
	assert!(!g.has_edge("a", "b"));
	let bar = g
		.edge_full("a", "b", Some("bar"))
		.unwrap();
	let foo = g
		.edge_full("a", "b", Some("foo"))
		.unwrap();
	assert_eq!(
		bar.points.clone().unwrap_or_default(),
		vec![Point { x: 5.0, y: 10.0 }]
	);
	assert_eq!(
		foo.points.clone().unwrap_or_default(),
		vec![Point { x: 15.0, y: 20.0 }]
	);
}
