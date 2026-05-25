use std::collections::HashMap;

use crate::{Edge, Graph, Path};

/// This function is an implementation of the Floyd-Warshall algorithm, which finds the
/// shortest path from each node to every other reachable node in the graph. It is similar
/// to alg.dijkstraAll, but it handles negative edge weights and is more efficient for some types
/// of graphs. This function returns a map of source -> { target -> { distance, predecessor }.
/// The distance property holds the sum of the weights from source to target along the shortest
/// path of Number.POSITIVE_INFINITY if there is no path from source. The predecessor property
/// can be used to walk the individual elements of the path from source to target in reverse
/// order.
///
/// Complexity: O(|V|^3).
///
/// @param graph - graph where to search paths.
/// @param weightFn - function which takes edge e and returns the weight of it. If no weightFn
/// is supplied then each edge is assumed to have a weight of 1. This function throws an
/// Error if any of the traversed edges have a negative edge weight.
/// @param edgeFn - function which takes a node v and returns the ids of all edges incident to it
/// for the purposes of shortest path traversal. By default this function uses the graph.outEdges.
/// @returns shortest paths map.
pub fn floyd_warshall<
	GraphLabel,
	NodeLabel,
	EdgeLabel,
	WF: Fn(&Edge) -> i32,
	EF: Fn(&str) -> Vec<Edge>,
>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	wf: impl Into<Option<WF>>,
	ef: impl Into<Option<EF>>,
) -> HashMap<String, HashMap<String, Path>> {
	let d_w = |_: &_| 1;
	let d_e = |v: &str| {
		g.out_edges(v.to_string(), None)
			.unwrap()
	};
	match (wf.into(), ef.into()) {
		(Some(wf), Some(ef)) => inner(g, wf, ef),
		(Some(wf), None) => inner(g, wf, d_e),
		(None, Some(ef)) => inner(g, d_w, ef),
		(None, None) => inner(g, d_w, d_e),
	}
}

fn inner<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	weight_fn: impl Fn(&Edge) -> i32,
	edge_fn: impl Fn(&str) -> Vec<Edge>,
) -> HashMap<String, HashMap<String, Path>> {
	let mut results = HashMap::new();
	for v in g.nodes() {
		let mut r_v = HashMap::new();
		r_v.insert(v.clone(), Path::default());
		for w in g.nodes() {
			if v != w {
				r_v.insert(
					w,
					Path {
						distance: i32::MAX,
						predecessor: String::new(),
					},
				);
			}
		}
		for edge in edge_fn(&v) {
			let w = if edge.v == v {
				edge.w.clone()
			} else {
				edge.v.clone()
			};
			let d = weight_fn(&edge);
			r_v.insert(
				w,
				Path {
					distance: d,
					predecessor: v.clone(),
				},
			);
		}
		results.insert(v, r_v);
	}

	for k in g.nodes() {
		for i in g.nodes() {
			for j in g.nodes() {
				let row_k = &results[&k];
				let row_i = &results[&i];
				let ik = &row_i[&k];
				let kj = &row_k[&j];
				let ij = &row_i[&j];
				let alt_distance = ik.distance + kj.distance;
				if alt_distance < ij.distance {
					let ij_predecessor = kj.predecessor.clone();
					let ij = results
						.get_mut(&i)
						.unwrap()
						.get_mut(&j)
						.unwrap();
					ij.distance = alt_distance;
					ij.predecessor = ij_predecessor;
				}
			}
		}
	}

	results
}
