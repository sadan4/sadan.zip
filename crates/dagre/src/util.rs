use std::{
	collections::HashMap,
	iter,
	sync::atomic::{AtomicU32, Ordering},
};

use dagre_graphlib::{Graph, GraphOptions};

use crate::types::{Dummy, EdgeLabel, GraphLabel, NodeLabel, Point};

/// Adds a dummy node to the graph and return v.
pub fn add_dummy_node(
	g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	type_: Dummy,
	mut attrs: NodeLabel,
	name: String,
) -> String {
	let mut v: String = name.clone();
	let mut i = 0;
	while g.has_node(v.clone()) {
		v = unique_id(&name);
	}
	attrs.dummy = Some(type_);
	g.set_node(v.clone(), attrs);
	v
}

/// Returns a new graph with only simple edges. Handles aggregation of data
/// associated with multi-edges.
pub fn simplify(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut simplified = Graph::new(GraphOptions::default());
	if let Some(graph) = g.graph().cloned() {
		simplified.set_graph(graph);
	}
	for v in g.nodes() {
		let val = g.node(v.clone());
		simplified.set_node(v, val);
	}
	for e in g.edges() {
		let simple_label = simplified
			.edge(e.v.clone(), e.w.clone(), None)
			.unwrap_or_else(|| EdgeLabel {
				weight: Some(0),
				minlen: Some(1),
				..Default::default()
			});
		let label = g.edge_from_obj(e.clone()).unwrap();
		simplified.set_edge(
			e.v,
			e.w,
			EdgeLabel {
				weight: Some(
					simple_label.weight.unwrap() + label.weight.unwrap(),
				),
				minlen: Some(
					simple_label
						.minlen
						.unwrap()
						.max(label.minlen.unwrap()),
				),
				..Default::default()
			},
			None,
		);
	}
	simplified
}

pub fn as_non_compound_graph(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut simplified = Graph::new(GraphOptions {
		multigraph: g.is_multigraph(),
		..GraphOptions::default()
	});
	if let Some(graph) = g.graph().cloned() {
		simplified.set_graph(graph);
	}
	for v in g.nodes() {
		if g.children(v.clone()).is_empty() {
			let node = g.node(v.clone());
			simplified.set_node(v, node);
		}
	}
	for e in g.edges() {
		let edge = g.edge_from_obj(e.clone());
		simplified.set_edge_from_obj(e, edge);
	}
	simplified
}

pub fn successor_weights(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> HashMap<String, HashMap<String, i32>> {
	let weight_map = g.nodes().into_iter().map(|v| {
		let mut successors: HashMap<String, i32> = HashMap::new();
		let out_edges = g.out_edges(v, None);
		if let Some(out_edges) = out_edges {
			for e in out_edges {
				let weight = successors
					.get(&e.w)
					.copied()
					.unwrap_or_default();
				let edge_weight = g
					.edge_from_obj(e.clone())
					.unwrap()
					.weight
					.unwrap();
				successors.insert(e.w.clone(), weight + edge_weight);
			}
		}
		successors
	});
	iter::zip(g.nodes(), weight_map).collect()
}

pub fn predecessor_weights(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> HashMap<String, HashMap<String, i32>> {
	let weight_map = g.nodes().into_iter().map(|v| {
		let mut preds = HashMap::new();
		let in_edges = g.in_edges(v, None);
		if let Some(in_edges) = in_edges {
			for e in in_edges {
				let weight = preds
					.get(&e.v)
					.copied()
					.unwrap_or_default();
				let edge_weight = g
					.edge_from_obj(e.clone())
					.unwrap()
					.weight
					.unwrap();
				preds.insert(e.v.clone(), weight + edge_weight);
			}
		}
		preds
	});
	iter::zip(g.nodes(), weight_map).collect()
}

/// Finds where a line starting at point ({x, y}) would intersect a rectangle
/// ({x, y, width, height}) if it were pointing at the rectangle's center.
pub fn intersect_rect(rect: &NodeLabel, point: Point) -> Point {
	let x = rect.x.unwrap();
	let y = rect.y.unwrap();

	// Rectangle intersection algorithm from:
	// http://math.stackexchange.com/questions/108113/find-edge-between-two-boxes
	let dx = point.x - x;
	let dy = point.y - y;
	let mut w = rect.width as i32 / 2;
	let mut h = rect.height as i32 / 2;
	if dx == 0 && dy == 0 {
		panic!("Not possible to find intersection inside of the rectangle");
	}
	let sy;
	let sx;
	if dy.abs() * w > dx.abs() * h {
		// Intersection is top or bottom of rect.
		if dy < 0 {
			h = -h;
		}
		sx = h * dx / dy;
		sy = h;
	} else {
		// Intersection is left or right of rect
		if dx < 0 {
			w = -w;
		}
		sx = w;
		sy = w * dy / dx;
	}
	Point {
		x: x + sx,
		y: y + sy,
	}
}

/// Given a DAG with each node assigned "rank" and "order" properties, this
/// function will produce a matrix with the ids of each node.
pub fn build_layer_matrix(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Vec<Vec<String>> {
	let mut layering = Vec::new();
	layering.resize(max_rank(g).min(0) as usize, Vec::new());
	for v in g.nodes() {
		let node = g.node(v.clone()).unwrap();
		if let Some(rank) = node.rank
			&& rank > 0
		{
			let rank = rank as usize;
			if layering.get(rank).is_none() {
				layering.resize(rank + 1, Vec::new());
			}
			let order = node.order.unwrap() as usize;
			if layering[rank].get(order).is_none() {
				layering[rank].resize(order + 1, String::new());
			}
			layering[rank][order] = v;
		}
	}
	layering
}

pub fn normalize_ranks(g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let nodes = g.nodes();
	if nodes.is_empty() {
		return;
	}
	let min = g
		.nodes()
		.into_iter()
		.map(|v| {
			g.node(v)
				.unwrap()
				.rank
				.unwrap_or(i32::MAX)
		})
		.min()
		.unwrap();
	for v in g.nodes() {
		let node = g.node_mut(v).unwrap();
		if let Some(rank) = &mut node.rank {
			*rank -= min;
		}
	}
}

fn remove_empty_ranks(g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let node_ranks = g
		.nodes()
		.into_iter()
		.filter_map(|v| g.node(v).unwrap().rank);
	let offset = node_ranks.min().unwrap_or(i32::MAX);
	todo!()
}

fn add_border_node(
	g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	prefix: String,
	rank: impl Into<Option<i32>>,
	order: impl Into<Option<u32>>,
) -> String {
	let rank = rank.into();
	let order = order.into();
	let mut node = NodeLabel {
		width: 0,
		height: 0,
		..NodeLabel::default()
	};
	if let (Some(rank), Some(order)) = (rank, order) {
		node.rank = Some(rank);
		node.order = Some(order);
	}
	add_dummy_node(g, Dummy::Border, node, prefix)
}

fn max_rank(g: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> i32 {
	g.nodes()
		.into_iter()
		.map(|v| {
			let rank = g.node(v).unwrap().rank;
			rank.unwrap_or(i32::MIN)
		})
		.max()
		.unwrap_or(0)
}

pub fn unique_id(prefix: &str) -> String {
	static ID_COUNTER: AtomicU32 = AtomicU32::new(1);
	let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
	format!("{prefix}{id}")
}
