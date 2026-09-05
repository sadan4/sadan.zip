//! Port of `lib/acyclic.ts`: reverse feedback-arc-set edges so the graph
//! becomes acyclic, then undo the reversal at the end of layout.

use crate::{
	graph::{Edge, Graph, NodeIdx},
	greedy_fas,
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

pub fn run(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let use_greedy = graph
		.graph()
		.and_then(|g| g.acyclicer.as_deref())
		.is_some_and(|s| s == "greedy");

	let fas: Vec<Edge> = if use_greedy {
		greedy_fas::greedy_fas(graph, |e| {
			graph
				.edge_obj(e)
				.map_or(1.0, |l| l.weight)
		})
	} else {
		dfs_fas(graph)
	};

	for e in fas {
		let mut label = match graph.edge_obj(&e) {
			Some(l) => l.clone(),
			None => continue,
		};
		graph.remove_edge_obj(&e);
		label.forward_name = e.name;
		label.reversed = true;
		let name = graph.fresh_edge_name();
		graph.set_edge_named(e.w, e.v, label, Some(name));
	}
}

fn dfs_fas(graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> Vec<Edge> {
	let mut fas: Vec<Edge> = Vec::new();
	// Node ids are dense indices, so the DFS state is a flat bitmap rather
	// than a hash set.
	let mut on_stack = vec![false; graph.node_bound()];
	let mut visited = vec![false; graph.node_bound()];

	fn dfs(
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		v: NodeIdx,
		on_stack: &mut [bool],
		visited: &mut [bool],
		fas: &mut Vec<Edge>,
	) {
		if visited[v.index()] {
			return;
		}
		visited[v.index()] = true;
		on_stack[v.index()] = true;
		if let Some(es) = graph.out_edges(v) {
			for e in es {
				if on_stack[e.w.index()] {
					fas.push(e);
				} else {
					dfs(graph, e.w, on_stack, visited, fas);
				}
			}
		}
		on_stack[v.index()] = false;
	}

	for v in graph.nodes() {
		dfs(graph, v, &mut on_stack, &mut visited, &mut fas);
	}
	fas
}

pub fn undo(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let edges = graph.edges();
	for e in edges {
		let mut label = match graph.edge_obj(&e) {
			Some(l) => l.clone(),
			None => continue,
		};
		if label.reversed {
			graph.remove_edge_obj(&e);
			let forward_name = label.forward_name.take();
			label.reversed = false;
			graph.set_edge_named(e.w, e.v, label, forward_name);
		}
	}
}
