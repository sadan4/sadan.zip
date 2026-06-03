//! Port of `lib/acyclic.ts`: reverse feedback-arc-set edges so the graph
//! becomes acyclic, then undo the reversal at the end of layout.

use crate::{
	graph::{Edge, Graph, NodeId},
	greedy_fas,
	types::{EdgeLabel, GraphLabel, NodeLabel},
	util::unique_id,
};

pub fn run(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let use_greedy = graph
		.graph()
		.and_then(|g| g.acyclicer.as_deref())
		.map(|s| s == "greedy")
		.unwrap_or(false);

	let fas: Vec<Edge> = if use_greedy {
		greedy_fas::greedy_fas(graph, |e| {
			graph
				.edge_obj(e)
				.map(|l| l.weight)
				.unwrap_or(1.0)
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
		label.forward_name = e.name.clone();
		label.reversed = true;
		let name = unique_id("rev");
		graph.set_edge_named(e.w.clone(), e.v.clone(), label, Some(name));
	}
}

fn dfs_fas(graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> Vec<Edge> {
	let mut fas: Vec<Edge> = Vec::new();
	let mut stack: std::collections::HashSet<NodeId> =
		std::collections::HashSet::new();
	let mut visited: std::collections::HashSet<NodeId> =
		std::collections::HashSet::new();

	fn dfs(
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		v: &str,
		stack: &mut std::collections::HashSet<NodeId>,
		visited: &mut std::collections::HashSet<NodeId>,
		fas: &mut Vec<Edge>,
	) {
		if visited.contains(v) {
			return;
		}
		visited.insert(v.into());
		stack.insert(v.into());
		if let Some(es) = graph.out_edges(v) {
			for e in es {
				if stack.contains(&e.w) {
					fas.push(e);
				} else {
					let w = e.w.clone();
					dfs(graph, &w, stack, visited, fas);
				}
			}
		}
		stack.remove(v);
	}

	for v in graph.nodes() {
		dfs(graph, &v, &mut stack, &mut visited, &mut fas);
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
			graph.set_edge_named(e.w.clone(), e.v.clone(), label, forward_name);
		}
	}
}
