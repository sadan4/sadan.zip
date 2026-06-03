//! Utility helpers — port of `lib/util.ts`.

use crate::{
	graph::{Edge, Graph, GraphOpts, NodeId},
	types::{Dummy, EdgeLabel, GraphLabel, NodeLabel, Point},
};
use std::{
	collections::HashMap,
	sync::atomic::{AtomicUsize, Ordering},
};

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn unique_id(prefix: &str) -> NodeId {
	let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
	format!("{prefix}{id}").into()
}

/// Reset the global id counter — useful in tests for determinism. Not
/// thread-safe across parallel tests; tests that need it run single-threaded.
#[cfg(test)]
pub fn reset_unique_id() {
	ID_COUNTER.store(0, Ordering::SeqCst);
}

pub fn add_dummy_node(
	g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	dummy_type: Dummy,
	mut attrs: NodeLabel,
	name: &str,
) -> NodeId {
	let mut v: NodeId = name.into();
	while g.has_node(&v) {
		v = unique_id(name);
	}
	attrs.dummy = Some(dummy_type);
	g.set_node(v.clone(), attrs);
	v
}

/// Returns a new graph with only simple edges (no multi-edges). Weights are
/// summed; minlen takes the max. Aggregates correspond to the JS `simplify`.
pub fn simplify(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut s: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	if let Some(g) = graph.graph() {
		s.set_graph(g.clone());
	}
	for v in graph.nodes() {
		if let Some(n) = graph.node(&v) {
			s.set_node(v, n.clone());
		}
	}
	for e in graph.edges() {
		let label = graph
			.edge_obj(&e)
			.cloned()
			.unwrap_or_default();
		let prev = s
			.edge(&e.v, &e.w)
			.cloned()
			.unwrap_or_else(|| EdgeLabel {
				weight: 0.0,
				minlen: 1,
				..Default::default()
			});
		s.set_edge(
			e.v.clone(),
			e.w.clone(),
			EdgeLabel {
				weight: prev.weight + label.weight,
				minlen: prev.minlen.max(label.minlen),
				..Default::default()
			},
		);
	}
	s
}

pub fn as_non_compound_graph(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut s: Graph<GraphLabel, NodeLabel, EdgeLabel> =
		Graph::with_opts(GraphOpts {
			directed: true,
			multigraph: graph.is_multigraph(),
			compound: false,
		});
	if let Some(g) = graph.graph() {
		s.set_graph(g.clone());
	}
	for v in graph.nodes() {
		if graph.children(Some(&v)).is_empty()
			&& let Some(n) = graph.node(&v)
		{
			s.set_node(v, n.clone());
		}
	}
	for e in graph.edges() {
		let l = graph
			.edge_obj(&e)
			.cloned()
			.unwrap_or_default();
		s.set_edge_named(e.v.clone(), e.w.clone(), l, e.name.clone());
	}
	s
}

pub fn successor_weights(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> HashMap<NodeId, HashMap<NodeId, f64>> {
	let mut out = HashMap::new();
	for v in graph.nodes() {
		let mut sucs: HashMap<NodeId, f64> = HashMap::new();
		if let Some(es) = graph.out_edges(&v) {
			for e in es {
				let w = graph
					.edge_obj(&e)
					.map_or(0.0, |l| l.weight);
				*sucs.entry(e.w.clone()).or_insert(0.0) += w;
			}
		}
		out.insert(v, sucs);
	}
	out
}

pub fn predecessor_weights(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> HashMap<NodeId, HashMap<NodeId, f64>> {
	let mut out = HashMap::new();
	for v in graph.nodes() {
		let mut preds: HashMap<NodeId, f64> = HashMap::new();
		if let Some(es) = graph.in_edges(&v) {
			for e in es {
				let w = graph
					.edge_obj(&e)
					.map_or(0.0, |l| l.weight);
				*preds.entry(e.v.clone()).or_insert(0.0) += w;
			}
		}
		out.insert(v, preds);
	}
	out
}

pub fn intersect_rect(rect: &NodeLabel, point: Point) -> Point {
	let x = rect.x.unwrap_or(0.0);
	let y = rect.y.unwrap_or(0.0);
	let dx = point.x - x;
	let dy = point.y - y;
	let mut w = rect.width / 2.0;
	let mut h = rect.height / 2.0;
	if dx == 0.0 && dy == 0.0 {
		panic!("Not possible to find intersection inside of the rectangle");
	}
	let (sx, sy);
	if dy.abs() * w > dx.abs() * h {
		if dy < 0.0 {
			h = -h;
		}
		sx = h * dx / dy;
		sy = h;
	} else {
		if dx < 0.0 {
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

pub fn build_layer_matrix(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Vec<Vec<NodeId>> {
	let mx = max_rank(graph);
	if mx < 0 {
		return Vec::new();
	}
	// Bucket each node into its rank, then sort each bucket by `order`. The
	// previous implementation indexed `layers[rank][order] = v` directly and
	// padded missing slots with `String::new()` — but `order` is sparse (gaps
	// from dummy removal etc.) and downstream code (`vertical_alignment`,
	// `build_block_graph`) treats those empty strings as real nodes, which
	// silently corrupts root/pos maps and causes nodes to share x positions.
	let mut layers: Vec<Vec<(usize, NodeId)>> = (0..=mx as usize)
		.map(|_| Vec::new())
		.collect();
	for v in graph.nodes_iter() {
		if let Some(n) = graph.node(v)
			&& let Some(rank) = n.rank
			&& rank >= 0
		{
			let r = rank as usize;
			if r >= layers.len() {
				layers.resize_with(r + 1, Vec::new);
			}
			layers[r].push((n.order.unwrap_or(0), v.into()));
		}
	}
	layers
		.into_iter()
		.map(|mut bucket| {
			bucket.sort_by_key(|(o, _)| *o);
			bucket
				.into_iter()
				.map(|(_, v)| v)
				.collect()
		})
		.collect()
}

pub fn max_rank(graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> i32 {
	let mut mx = i32::MIN;
	let mut seen = false;
	for v in graph.nodes_iter() {
		if let Some(n) = graph.node(v)
			&& let Some(r) = n.rank
		{
			seen = true;
			if r > mx {
				mx = r;
			}
		}
	}
	if seen { mx } else { -1 }
}

pub fn normalize_ranks(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let mut min = i32::MAX;
	for v in graph.nodes() {
		if let Some(n) = graph.node(&v)
			&& let Some(r) = n.rank
			&& r < min
		{
			min = r;
		}
	}
	if min == i32::MAX {
		return;
	}
	for v in graph.nodes() {
		if let Some(n) = graph.node_mut(&v)
			&& let Some(r) = n.rank
		{
			n.rank = Some(r - min);
		}
	}
}

pub fn remove_empty_ranks(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let ranks: Vec<i32> = graph
		.nodes()
		.iter()
		.filter_map(|v| graph.node(v).and_then(|n| n.rank))
		.collect();
	if ranks.is_empty() {
		return;
	}
	let offset = *ranks.iter().min().unwrap();
	let mut layers: Vec<Option<Vec<NodeId>>> = Vec::new();
	for v in graph.nodes() {
		if let Some(n) = graph.node(&v)
			&& let Some(r) = n.rank
		{
			let idx = (r - offset) as usize;
			while layers.len() <= idx {
				layers.push(None);
			}
			layers[idx]
				.get_or_insert_with(Vec::new)
				.push(v.clone());
		}
	}
	let factor = graph
		.graph()
		.and_then(|g| g.node_rank_factor)
		.unwrap_or(0.0) as i32;
	let mut delta = 0i32;
	for (i, vs) in layers.iter().enumerate() {
		match vs {
			None if factor != 0 && i as i32 % factor != 0 => {
				delta -= 1;
			}
			Some(vs) if delta != 0 => {
				for v in vs {
					if let Some(n) = graph.node_mut(v)
						&& let Some(r) = n.rank
					{
						n.rank = Some(r + delta);
					}
				}
			}
			_ => {}
		}
	}
}

pub fn add_border_node(
	graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	prefix: &str,
	rank: Option<i32>,
	order: Option<usize>,
) -> NodeId {
	let node = NodeLabel {
		width: 0.0,
		height: 0.0,
		rank,
		order,
		..Default::default()
	};
	add_dummy_node(graph, Dummy::Border, node, prefix)
}

pub fn range(start: i32, limit: i32, step: i32) -> Vec<i32> {
	let mut r = Vec::new();
	if step > 0 {
		let mut i = start;
		while i < limit {
			r.push(i);
			i += step;
		}
	} else if step < 0 {
		let mut i = start;
		while i > limit {
			r.push(i);
			i += step;
		}
	}
	r
}

pub fn range0(limit: i32) -> Vec<i32> {
	range(0, limit, 1)
}

pub fn partition<T, F: Fn(&T) -> bool>(
	items: Vec<T>,
	pred: F,
) -> (Vec<T>, Vec<T>) {
	let mut lhs = Vec::new();
	let mut rhs = Vec::new();
	for v in items {
		if pred(&v) {
			lhs.push(v);
		} else {
			rhs.push(v);
		}
	}
	(lhs, rhs)
}

/// Aggregate the weight of an edge — used by greedy-fas. Picks the `weight`
/// field, defaulting to 1 when an explicit weight function isn't provided.
pub fn edge_weight(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	e: &Edge,
) -> f64 {
	graph
		.edge_obj(e)
		.map_or(1.0, |l| l.weight)
}
