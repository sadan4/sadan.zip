use std::collections::HashSet;

use crate::Graph;

pub struct CycleError;

/// Given a graph this function applies topological sorting to it.
/// If the graph has a cycle it is impossible to generate such a list and [`CycleError`] is thrown.
/// Complexity: O(|V| + |E|).
///
/// @param graph - graph to apply topological sorting to.
/// @returns an array of nodes such that for each edge u -> v, u appears before v in the array.
pub fn topsort<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Result<Vec<String>, CycleError> {
	let mut visited = HashSet::new();
	let mut stack = HashSet::new();
	let mut result = Vec::new();
	fn visit<GraphLabel, NodeLabel, EdgeLabel>(
		g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		visited: &mut HashSet<String>,
		results: &mut Vec<String>,
		stack: &mut HashSet<String>,
		node: &str,
	) -> Result<(), CycleError> {
		if stack.contains(node) {
			return Err(CycleError);
		}
		if !visited.contains(node) {
			stack.insert(node.to_string());
			for predecessor in g
				.predecessors(node.to_string())
				.unwrap()
			{
				visit(g, visited, results, stack, &predecessor)?;
			}
			stack.remove(node);
			results.push(node.to_string());
		}

		Ok(())
	}
	for sink in g.sinks() {
		visit(g, &mut visited, &mut result, &mut stack, &sink)?;
	}

	if visited.len() == g.node_count() as usize {
		Ok(result)
	} else {
		Err(CycleError)
	}
}
