use std::collections::HashMap;

use crate::{Edge, Graph, GraphOptions, data};

/**
 * Prim's algorithm takes a connected undirected graph and generates a minimum spanning tree. This
 * function returns the minimum spanning tree as an undirected graph. This algorithm is derived
 * from the description in "Introduction to Algorithms", Third Edition, Cormen, et al., Pg 634.
 * Complexity: O(|E| * log |V|);
 *
 * @param graph - graph to generate a minimum spanning tree of.
 * @param weightFn - function which takes edge e and returns the weight of it. It throws an Error if
 * the graph is not connected.
 * @returns minimum spanning tree of graph.
 */
pub fn prim<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	weight_fn: impl Fn(&Edge) -> i32,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut result = Graph::new(GraphOptions::default());
	let mut parents: HashMap<String, String> = HashMap::new();
	let mut pq = data::PriorityQueue::new();
	let mut v: String;

	fn update_neighbors(
		edge: &Edge,
		weight_fn: impl Fn(&Edge) -> i32,
		v: &str,
		pq: &mut data::PriorityQueue,
		parents: &mut HashMap<String, String>,
	) {
		let w = if edge.v == v {
			edge.w.as_str()
		} else {
			edge.v.as_str()
		};
		let Some(pri) = pq.priority(w) else {
			return;
		};
		let edge_weight = weight_fn(edge);
		if edge_weight < pri {
			parents.insert(w.to_string(), v.to_string());
			pq.decrease(w, edge_weight);
		}
	}
	if g.node_count() == 0 {
		return result;
	}
	for v in g.nodes() {
		pq.add(v.clone(), i32::MAX);
		result.set_node(v, None);
	}
	// Start from an arbitrary node
	pq.decrease(&g.nodes()[0], 0);

	let mut init = false;

	while pq.size() != 0 {
		v = pq.remove_min();
		if parents.contains_key(&v) {
			result.set_edge(v.clone(), parents[&v].clone(), None, None);
		} else if init {
			panic!("Input graph is not connected");
		} else {
			init = true;
		}

		for edge in g.node_edges(v.clone(), None).unwrap() {
			update_neighbors(&edge, &weight_fn, &v, &mut pq, &mut parents);
		}
	}

	result
}
