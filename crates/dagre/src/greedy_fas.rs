//! Greedy feedback-arc-set heuristic (Eades, Lin, Smyth).
//!
//! Operates on the layout graph and uses an internal lightweight
//! "fas-graph" whose node label is `FasEntry` (in/out weight) and edge label
//! is the aggregated weight.

use crate::{
	graph::{Edge, Graph, GraphOpts},
	list::{FasEntry, List},
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

#[derive(Debug, Default, Clone)]
struct FasNode {
	v: String,
	in_w: f64,
	out_w: f64,
}

#[derive(Debug, Default, Clone)]
struct FasEdge {
	weight: f64,
}

type FasGraph = Graph<(), FasNode, FasEdge>;

struct State {
	g: FasGraph,
	buckets: Vec<List>,
	zero_idx: usize,
}

pub fn greedy_fas<F>(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	weight_fn: F,
) -> Vec<Edge>
where
	F: Fn(&Edge) -> f64,
{
	if graph.node_count() <= 1 {
		return Vec::new();
	}
	let mut state = build_state(graph, &weight_fn);
	let results = do_greedy_fas(&mut state);

	// Expand multi-edges to the actual edges in the original graph.
	let mut out: Vec<Edge> = Vec::new();
	for edge in results {
		if let Some(es) = graph.out_edges_to(&edge.v, &edge.w) {
			out.extend(es);
		}
	}
	out
}

fn do_greedy_fas(state: &mut State) -> Vec<Edge> {
	let mut results: Vec<Edge> = Vec::new();

	while state.g.node_count() > 0 {
		// Drain sinks (bucket 0).
		while let Some(entry) = state.buckets[0].dequeue() {
			remove_node(state, entry, false, &mut results);
		}
		// Drain sources (bucket last).
		let last = state.buckets.len() - 1;
		while let Some(entry) = state.buckets[last].dequeue() {
			remove_node(state, entry, false, &mut results);
		}
		if state.g.node_count() > 0 {
			for i in (1..state.buckets.len() - 1).rev() {
				if let Some(entry) = state.buckets[i].dequeue() {
					remove_node(state, entry, true, &mut results);
					break;
				}
			}
		}
	}
	results
}

fn remove_node(
	state: &mut State,
	entry: FasEntry,
	collect_predecessors: bool,
	results: &mut Vec<Edge>,
) {
	let v = entry.v.clone();

	let in_es = state.g.in_edges(&v).unwrap_or_default();
	for e in in_es {
		let w = state
			.g
			.edge_obj(&e)
			.map(|l| l.weight)
			.unwrap_or(0.0);
		if collect_predecessors {
			results.push(Edge::new(e.v.clone(), e.w.clone()));
		}
		// mutate u's out weight
		if let Some(u_node) = state.g.node_mut(&e.v) {
			u_node.out_w -= w;
		}
		// rebucket u
		let u_copy = state.g.node(&e.v).cloned();
		if let Some(u) = u_copy {
			assign_bucket(&mut state.buckets, state.zero_idx, &u);
		}
	}

	let out_es = state
		.g
		.out_edges(&v)
		.unwrap_or_default();
	for e in out_es {
		let w = state
			.g
			.edge_obj(&e)
			.map(|l| l.weight)
			.unwrap_or(0.0);
		if let Some(w_node) = state.g.node_mut(&e.w) {
			w_node.in_w -= w;
		}
		let w_copy = state.g.node(&e.w).cloned();
		if let Some(wnode) = w_copy {
			assign_bucket(&mut state.buckets, state.zero_idx, &wnode);
		}
	}

	state.g.remove_node(&v);
}

fn assign_bucket(buckets: &mut [List], zero_idx: usize, entry: &FasNode) {
	// First remove from whichever bucket currently holds this v.
	for b in buckets.iter_mut() {
		if b.remove(&entry.v).is_some() {
			break;
		}
	}
	let fe = FasEntry {
		v: entry.v.clone(),
		in_w: entry.in_w,
		out_w: entry.out_w,
	};
	if entry.out_w == 0.0 {
		buckets[0].enqueue(fe);
	} else if entry.in_w == 0.0 {
		let last = buckets.len() - 1;
		buckets[last].enqueue(fe);
	} else {
		let idx = (entry.out_w - entry.in_w) as i64 + zero_idx as i64;
		let idx = idx.clamp(1, buckets.len() as i64 - 2) as usize;
		buckets[idx].enqueue(fe);
	}
}

fn build_state<F>(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	weight_fn: &F,
) -> State
where
	F: Fn(&Edge) -> f64,
{
	let mut g: FasGraph = Graph::with_opts(GraphOpts::directed());
	let mut max_in = 0.0_f64;
	let mut max_out = 0.0_f64;

	for v in graph.nodes() {
		g.set_node(
			v.clone(),
			FasNode {
				v: v.clone(),
				in_w: 0.0,
				out_w: 0.0,
			},
		);
	}
	for e in graph.edges() {
		let prev = g
			.edge(&e.v, &e.w)
			.map(|l| l.weight)
			.unwrap_or(0.0);
		let w = weight_fn(&e);
		let combined = prev + w;
		g.set_edge(e.v.clone(), e.w.clone(), FasEdge { weight: combined });
		if let Some(vn) = g.node_mut(&e.v) {
			vn.out_w += w;
			if vn.out_w > max_out {
				max_out = vn.out_w;
			}
		}
		if let Some(wn) = g.node_mut(&e.w) {
			wn.in_w += w;
			if wn.in_w > max_in {
				max_in = wn.in_w;
			}
		}
	}
	let n_buckets = max_out as usize + max_in as usize + 3;
	let buckets: Vec<List> = (0..n_buckets)
		.map(|_| List::new())
		.collect();
	let zero_idx = max_in as usize + 1;
	let mut state = State {
		g,
		buckets,
		zero_idx,
	};
	let nodes: Vec<String> = state.g.nodes();
	for v in nodes {
		let node = state.g.node(&v).cloned().unwrap();
		assign_bucket(&mut state.buckets, state.zero_idx, &node);
	}
	state
}
