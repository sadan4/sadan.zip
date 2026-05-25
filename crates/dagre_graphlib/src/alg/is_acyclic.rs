use crate::{Graph, alg};

/// Given a Graph, graph, this function returns true if the graph has no cycles and returns false if it
/// does. This algorithm returns as soon as it detects the first cycle. You can use [`alg::find_cycles`]
/// to get the actual list of cycles in the graph.
///
/// @param graph - graph to detect whether it acyclic or not.
/// @returns whether graph contain cycles or not.
pub fn is_acyclic<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> bool {
	alg::topsort(g).is_ok()
}
