use std::{collections::HashSet, mem};

use crate::Graph;

/// Finds all connected components in a graph and returns an array of these components.
/// Each component is itself an array that contains the ids of nodes in the component.
/// Complexity: O(|V|).
///
/// @param graph - graph to find components in.
/// @returns array of nodes list representing components
pub fn components<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Vec<Vec<String>> {
	let mut visited: HashSet<String> = HashSet::new();
	let mut components_arr: Vec<Vec<String>> = Vec::new();
	let mut components: Vec<String> = Vec::new();

	fn dfs<GraphLabel, NodeLabel, EdgeLabel>(
		g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		components: &mut Vec<String>,
		visited: &mut HashSet<String>,
		v: &str,
	) {
		if visited.contains(v) {
			return;
		}
		visited.insert(v.to_string());
		components.push(v.to_string());
		for v in g.successors(v.to_string()).unwrap() {
			dfs(g, components, visited, &v);
		}
		for v in g.predecessors(v.to_string()).unwrap() {
			dfs(g, components, visited, &v);
		}
	}

	for v in g.nodes() {
		dfs(g, &mut components, &mut visited, &v);
		if !components.is_empty() {
			components_arr.push(mem::take(&mut components));
		}
	}

	components_arr
}
