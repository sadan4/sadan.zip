use std::collections::HashMap;

use crate::{Edge, Graph, Path, data};

/// This function is an implementation of Dijkstra's algorithm which finds the shortest
/// path from source to all other nodes in graph. This function returns a map of
/// v -> { distance, predecessor }. The distance property holds the sum of the weights
/// from source to v along the shortest path or Number.POSITIVE_INFINITY if there is no path
/// from source. The predecessor property can be used to walk the individual elements of the
/// path from source to v in reverse order.
/// Complexity: O((|E| + |V|) * log |V|).
///
/// @param graph - graph where to search paths.
/// @param source - node to start paths from.
/// @param weightFn - function which takes edge e and returns the weight of it. If no weightFn
/// is supplied then each edge is assumed to have a weight of 1. This function throws an
/// Error if any of the traversed edges have a negative edge weight.
/// @param edgeFn - function which takes a node v and returns the ids of all edges incident to it
/// for the purposes of shortest path traversal. By default this function uses the graph.outEdges.
/// @returns shortest paths map that starts from node source
pub fn dijkstra<
	GraphLabel,
	NodeLabel,
	EdgeLabel,
	WF: Fn(&Edge) -> i32,
	EF: Fn(&str) -> Vec<Edge>,
>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	source: &str,
	wf: impl Into<Option<WF>>,
	ef: impl Into<Option<EF>>,
) -> HashMap<String, Path> {
	let default_weight_fn = |_: &_| 1;
	let default_edge_fn = |v: &str| {
		g.out_edges(v.to_string(), None)
			.unwrap()
	};

	match (wf.into(), ef.into()) {
		(Some(wf), Some(ef)) => inner(g, source, wf, ef),
		(Some(wf), None) => inner(g, source, wf, default_edge_fn),
		(None, Some(ef)) => inner(g, source, default_weight_fn, ef),
		(None, None) => inner(g, source, default_weight_fn, default_edge_fn),
	}
}

fn inner<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	source: &str,
	weight_fn: impl Fn(&Edge) -> i32,
	edge_fn: impl Fn(&str) -> Vec<Edge>,
) -> HashMap<String, Path> {
	let mut results = HashMap::new();
	let mut pq = data::PriorityQueue::new();
	let mut v: String;
	let mut v_entry: Path;
	fn update_neighbors(
		edge: &Edge,
		v: &str,
		results: &mut HashMap<String, Path>,
		weight_fn: impl Fn(&Edge) -> i32,
		v_entry: &Path,
		pq: &mut data::PriorityQueue,
	) {
		let w = if edge.v == v {
  			edge.w.as_str()
  		} else {
  			edge.v.as_str()
  		};
		let w_entry = results.get_mut(w).unwrap();
		let weight = weight_fn(edge);
		let distance = v_entry.distance + weight;
		if weight < 0 {
			panic!("dijkstra does not allow negative edge weights. Bad edge: {edge:?} Weight: {weight}");
		}
		if distance < w_entry.distance {
			w_entry.distance = distance;
			w_entry.predecessor = v.to_string();
			pq.decrease(w, distance);
		}
	}

	for v in g.nodes() {
		let distance = if v == source {
			0
		} else {
			i32::MAX
		};
		results.insert(v.clone(), Path {distance, predecessor: String::new()});
		pq.add(v, distance);
	}

	while pq.size() != 0 {
		v = pq.remove_min();
		v_entry = results.get(&v).unwrap().clone();
		if v_entry.distance == i32::MAX {
			break;
		}
		for edge in edge_fn(&v) {
			update_neighbors(&edge, &v, &mut results, &weight_fn, &v_entry, &mut pq);
		}
	}

	results
}
