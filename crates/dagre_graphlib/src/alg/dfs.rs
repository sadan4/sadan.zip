use crate::{Graph, alg};

/// Pre- or post-order traversal on the input graph.
///
/// Returns an array of the nodes in the order they were visited.
pub fn dfs<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	vs: Vec<String>,
	order: alg::Order,
) -> Vec<String> {
	alg::reduce(
		g,
		vs,
		order,
		|mut acc, v| {
			acc.push(v);
			acc
		},
		Vec::new(),
	)
}
