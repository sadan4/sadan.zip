use std::collections::HashMap;

use crate::{Edge, Graph, Path};

pub fn bellman_ford<
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

pub fn inner<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	source: &str,
	weight_fn: impl Fn(&Edge) -> i32,
	edge_fn: impl Fn(&str) -> Vec<Edge>,
) -> HashMap<String, Path> {
	let mut results: HashMap<String, Path> = HashMap::new();
	let mut did_a_distance_upgrade;
	let mut iterations = 0;
	let nodes = g.nodes();

	macro_rules! relax_edge {
		($edge:expr) => {{
			let edge: &Edge = $edge;
			let edge_weight = weight_fn(edge);
			let distance = results[&edge.v].distance + edge_weight;
			if distance < results[&edge.w].distance {
				*results.get_mut(&edge.w).unwrap() = Path {
					distance,
					predecessor: edge.v.clone(),
				};
				did_a_distance_upgrade = true;
			}
		}};
	}

	macro_rules! relax_all_edges {
		() => {{
			for vertex in &nodes {
				for edge in edge_fn(vertex) {
					// If the vertex on which the edgeFun in called is
					// the edge.w, then we treat the edge as if it was reversed

					let in_vertex = if edge.v == *vertex {
						edge.v.clone()
					} else {
						edge.w.clone()
					};
					let out_vertex =
						if in_vertex == edge.v { edge.w } else { edge.v };
					relax_edge!(&Edge {
						v: in_vertex,
						w: out_vertex,
						name: None,
					})
				}
			}
		}};
	}

	// Initialization
	for node in &nodes {
		let distance = if node == source { 0 } else { i32::MAX };
		results.insert(
			node.clone(),
			Path {
				distance,
				predecessor: String::new(),
			},
		);
	}

	let number_of_nodes = nodes.len();

	// Relax all edges in |V|-1 iterations
	for _ in 0..number_of_nodes - 1 {
		did_a_distance_upgrade = false;
		iterations += 1;
		relax_all_edges!();
		if !did_a_distance_upgrade {
			// Ιf no update was made in an iteration, Bellman-Ford has finished
			break;
		}
	}

	// Detect if the graph contains a negative weight cycle
	if iterations == number_of_nodes - 1 {
		did_a_distance_upgrade = false;
		relax_all_edges!();
		if did_a_distance_upgrade {
			panic!("Graph contains a negative weight cycle");
		}
	}

	results
}
