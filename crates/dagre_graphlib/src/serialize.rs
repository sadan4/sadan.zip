use serde::{Deserialize, Serialize};

use crate::{Edge, Graph, types::GraphOptions};

#[derive(Serialize, Deserialize)]
pub struct SerializedGraph<GraphLabel, NodeLabel, EdgeLabel> {
	options: GraphOptions,
	nodes: Vec<SerializedNode<NodeLabel>>,
	edges: Vec<SerializedEdge<EdgeLabel>>,
	value: Option<GraphLabel>,
}

#[derive(Serialize, Deserialize)]
struct SerializedNode<NodeLabel> {
	v: String,
	value: NodeLabel,
	parent: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SerializedEdge<EdgeLabel> {
	v: String,
	w: String,
	name: Option<String>,
	value: EdgeLabel,
}
/// Creates a JSON representation of the graph that can be serialized to a string with
/// JSON.stringify. The graph can later be restored using json.read.
///
/// @param graph - target to create JSON representation of.
/// @returns JSON serializable graph representation
pub fn serialize_graph<GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> SerializedGraph<GraphLabel, NodeLabel, EdgeLabel>
where
	GraphLabel: Serialize + Clone,
	NodeLabel: Serialize + Clone,
	EdgeLabel: Serialize + Clone,
{
	let write_nodes = || -> Vec<_> {
		g.nodes()
			.into_iter()
			.map(|v: String| {
				let node_value = g.node(v.clone()).unwrap();
				let parent = g.parent(v.clone());
				SerializedNode {
					v,
					parent,
					value: node_value,
				}
			})
			.collect()
	};
	let write_edges = || -> Vec<_> {
		g.edges()
			.into_iter()
			.map(|e| {
				let edge_value = g.edge_from_obj(e.clone()).unwrap();
				SerializedEdge {
					v: e.v,
					w: e.w,
					name: e.name,
					value: edge_value,
				}
			})
			.collect()
	};
	SerializedGraph {
		options: GraphOptions {
			directed: g.is_directed(),
			multigraph: g.is_multigraph(),
			compound: g.is_compound(),
		},
		nodes: write_nodes(),
		edges: write_edges(),
		value: g.graph().cloned(),
	}
}

pub fn deserialize_graph<GraphLabel, NodeLabel, EdgeLabel>(
	SerializedGraph {
		options,
		nodes,
		edges,
		value,
	}: SerializedGraph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel>
where
	GraphLabel: Deserialize<'static>,
	NodeLabel: Deserialize<'static>,
	EdgeLabel: Deserialize<'static>,
{
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new(options);
	if let Some(graph_label) = value {
		g.set_graph(graph_label);
	}
	for entry in nodes {
		g.set_node(entry.v.clone(), entry.value);
		if let Some(parent) = entry.parent {
			g.set_parent(entry.v, parent);
		}
	}
	for entry in edges {
		g.set_edge_from_obj(
			Edge {
				v: entry.v,
				w: entry.w,
				name: entry.name,
			},
			entry.value,
		);
	}
	g
}
