//! Port of `lib/normalize.ts`. Splits long edges (those spanning multiple
//! ranks) into chains of length-1 edges with dummy nodes, then can undo to
//! reconstruct points/labels.

use crate::{
	graph::{Edge, Graph, NodeId},
	types::{Dummy, EdgeLabel, GraphLabel, NodeLabel, Point},
	util::add_dummy_node,
};

pub fn run(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	if let Some(gl) = graph.graph_mut() {
		gl.dummy_chains = Some(Vec::new());
	}
	let edges = graph.edges();
	for e in edges {
		normalize_edge(graph, &e);
	}
}

fn normalize_edge(
	graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	e: &Edge,
) {
	let v_rank = graph
		.node(&e.v)
		.and_then(|n| n.rank)
		.expect("v has rank");
	let w_rank = graph
		.node(&e.w)
		.and_then(|n| n.rank)
		.expect("w has rank");
	if w_rank == v_rank + 1 {
		return;
	}
	let name = e.name.clone();
	let edge_label = graph
		.edge_obj(e)
		.cloned()
		.unwrap_or_default();
	let label_rank = edge_label.label_rank;

	graph.remove_edge_obj(e);

	let mut v = e.v.clone();
	let mut cur_rank = v_rank + 1;
	let mut i = 0;
	while cur_rank < w_rank {
		let mut attrs = NodeLabel {
			width: 0.0,
			height: 0.0,
			edge_label: Some(Box::new(EdgeLabel {
				points: Some(Vec::new()),
				..edge_label.clone()
			})),
			edge_obj: Some(e.clone()),
			rank: Some(cur_rank),
			..Default::default()
		};
		let mut typ = Dummy::Edge;
		if Some(cur_rank) == label_rank {
			attrs.width = edge_label.width;
			attrs.height = edge_label.height;
			typ = Dummy::EdgeLabel;
			attrs.labelpos = edge_label.labelpos;
		}
		let dummy = add_dummy_node(graph, typ, attrs, "_d");
		graph.set_edge_named(
			v.clone(),
			dummy.clone(),
			EdgeLabel {
				weight: edge_label.weight,
				..Default::default()
			},
			name.clone(),
		);
		if i == 0
			&& let Some(gl) = graph.graph_mut()
		{
			gl.dummy_chains
				.get_or_insert_with(Vec::new)
				.push(dummy.clone());
		}
		v = dummy;
		i += 1;
		cur_rank += 1;
	}
	graph.set_edge_named(
		v,
		e.w.clone(),
		EdgeLabel {
			weight: edge_label.weight,
			..Default::default()
		},
		name,
	);
}

pub fn undo(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let chains: Vec<NodeId> = graph
		.graph()
		.and_then(|g| g.dummy_chains.clone())
		.unwrap_or_default();

	for start in chains {
		let mut v = start;
		// Read first dummy's original edge object + label.
		let (orig_obj, mut orig_label) = match graph.node(&v) {
			Some(n) => match (&n.edge_obj, &n.edge_label) {
				(Some(o), Some(l)) => (o.clone(), (**l).clone()),
				_ => continue,
			},
			None => continue,
		};
		// Reinstate the original edge.
		graph.set_edge_named(
			orig_obj.v.clone(),
			orig_obj.w.clone(),
			orig_label.clone(),
			orig_obj.name.clone(),
		);

		while let Some(node) = graph.node(&v) {
			if node.dummy.is_none() {
				break;
			}
			let Some(w) = graph
				.successors(&v)
				.and_then(|s| s.into_iter().next())
			else {
				break;
			};
			let node = node.clone();
			graph.remove_node(&v);
			orig_label
				.points
				.get_or_insert_with(Vec::new)
				.push(Point {
					x: node.x.unwrap_or(0.0),
					y: node.y.unwrap_or(0.0),
				});
			if node.dummy == Some(Dummy::EdgeLabel) {
				orig_label.x = node.x;
				orig_label.y = node.y;
				orig_label.width = node.width;
				orig_label.height = node.height;
			}
			v = w;
		}

		// Re-set the edge label (since `set_edge_named` above replaces with
		// initial clone; we've accumulated points/coords).
		graph.set_edge_named(
			orig_obj.v.clone(),
			orig_obj.w.clone(),
			orig_label,
			orig_obj.name.clone(),
		);
	}
}
