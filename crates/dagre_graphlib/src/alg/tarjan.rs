use std::collections::HashMap;

use crate::Graph;

struct VisitedEntry {
	on_stack: bool,
	low_link: u32,
	index: u32,
}

/// This function is an implementation of Tarjan's algorithm which finds all strongly connected
/// components in the directed graph g. Each strongly connected component is composed of nodes that
/// can reach all other nodes in the component via directed edges. A strongly connected component
/// can consist of a single node if that node cannot both reach and be reached by any other
/// specific node in the graph. Components of more than one node are guaranteed to have at least
/// one cycle.
/// Complexity: O(|V| + |E|).
///
/// @param graph - graph to find all strongly connected components of.
/// @returns an array of components. Each component is itself an array that contains
/// the ids of all nodes in the component.
pub fn tarjan<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Vec<Vec<String>> {
	let mut index = 0;
	let mut stack = Vec::new();
	let mut visited = HashMap::new();
	let mut results = Vec::new();
	fn dfs<GraphLabel, NodeLabel, EdgeLabel>(
		g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		visited: &mut HashMap<String, VisitedEntry>,
		index: &mut u32,
		stack: &mut Vec<String>,
		results: &mut Vec<Vec<String>>,
		v: &str,
	) {
		visited.insert(
			v.to_string(),
			VisitedEntry {
				on_stack: true,
				low_link: *index,
				index: *index + 1,
			},
		);

		*index += 1;

		stack.push(v.to_string());

		for w in g
			.successors(v.to_string())
			.unwrap_or_default()
		{
			if !visited.contains_key(&w) {
				dfs(g, visited, index, stack, results, &w);
				let visited_w_low_link = visited[&w].low_link;
				let entry = visited.get_mut(v).unwrap();
				entry.low_link = entry.low_link.min(visited_w_low_link);
			} else if visited[&w].on_stack {
				let visited_w_index = visited[&w].index;
				let entry = visited.get_mut(v).unwrap();
				entry.low_link = entry.low_link.min(visited_w_index);
			}
		}
		let entry = &visited[v];
		if entry.low_link == entry.index {
			let mut components = Vec::new();
			let mut w: String = stack.pop().unwrap();
			loop {
				visited.get_mut(&w).unwrap().on_stack = false;
				if v == w {
					components.push(w);
					break;
				}
				components.push(w);
				w = stack.pop().unwrap();
			}
			results.push(components);
		}
	}

	for v in g.nodes() {
		if !visited.contains_key(&v) {
			dfs(g, &mut visited, &mut index, &mut stack, &mut results, &v);
		}
	}

	results
}
