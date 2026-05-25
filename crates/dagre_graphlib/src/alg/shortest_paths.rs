use std::collections::HashMap;

use crate::{Edge, Graph, Path, alg};

pub fn shortest_paths<
	GraphLabel,
	NodeLabel,
	EdgeLabel,
	WF: Fn(&Edge) -> i32,
	EF: Fn(&str) -> Vec<Edge>,
>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	source: &str,
	weight_fn: impl Into<Option<WF>>,
	edge_fn: impl Into<Option<EF>>,
) -> HashMap<String, Path> {
	let default_edge_fn = |v: &str| {
		g.out_edges(v.to_string(), None)
			.unwrap_or_default()
	};
	let weight_fn = weight_fn.into();
	match edge_fn.into() {
		Some(edge_fn) => inner(g, source, weight_fn, edge_fn),
		None => inner(g, source, weight_fn, default_edge_fn),
	}
}

fn inner<
	GraphLabel,
	NodeLabel,
	EdgeLabel,
	WF: Fn(&Edge) -> i32,
	EF: Fn(&str) -> Vec<Edge>,
>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	source: &str,
	weight_fn: Option<WF>,
	edge_fn: EF,
) -> HashMap<String, Path> {
	let Some(weight_fn) = weight_fn else {
		return alg::dijkstra::<
			GraphLabel,
			NodeLabel,
			EdgeLabel,
			fn(&Edge) -> i32,
			EF,
		>(g, source, None, edge_fn);
	};
	let mut negative_edge_exists = false;
	let nodes = g.nodes();

	for i in 0..nodes.len() {
		let adj_list = edge_fn(&nodes[i]);

		for j in 0..adj_list.len() {
			let edge = &adj_list[j];
			let in_vertex = if edge.v == nodes[i] {
				edge.v.as_str()
			} else {
				edge.w.as_str()
			}
			.to_string();
			let out_vertex = if in_vertex == edge.v {
				edge.w.as_str()
			} else {
				edge.v.as_str()
			}
			.to_string();
			if weight_fn(&Edge {
				v: in_vertex,
				w: out_vertex,
				name: None,
			}) < 0
			{
				negative_edge_exists = true;
			}
		}
		if negative_edge_exists {
			return alg::bellman_ford(g, source, weight_fn, edge_fn);
		}
	}

	alg::dijkstra(g, source, weight_fn, edge_fn)
}
