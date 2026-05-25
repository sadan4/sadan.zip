use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct GraphOptions {
	/// Whether the graph edges have an orientation.
	///
	/// Default: `true`
	pub directed: bool,
	/// Whether the pair of nodes of the graph can have multiple edges.
	///
	/// Default: `false`
	pub multigraph: bool,
	/// Whether a node of the graph can have subnodes.
	///
	/// Default: `false`
	pub compound: bool,
}

/// Represents an edge in the graph
#[derive(Clone, Debug)]
pub struct Edge {
	/// Source node identifier
	pub v: String,
	/// Target node identifier
	pub w: String,
	/// The name that uniquely identifies a multi-edge
	pub name: Option<String>,
}
/// Represents a path in the graph with distance and predecessor information.
#[derive(Default, Clone)]
pub struct Path {
	/// The sum of weights from source to this node along the shortest path
	pub distance: i32,
	/// The predecessor node in the shortest path, used to walk back to the source
	pub predecessor: String,
}

/// Function that takes an edge and returns its weight
pub type WeightFunction<'a> = dyn 'a + Fn(&Edge) -> u32;

/// Function that takes a node and returns the edges incident to it
pub type EdgeFunction<'a> = dyn 'a + Fn(&str) -> Vec<Edge>;

/// Factory function that creates a label for a node
pub trait NodeLabelFactory<NodeLabel> {
	fn create_node_label(&self, v: String) -> NodeLabel;
}

/// Factory function that creates a label for an edge
pub trait EdgeLabelFactory<EdgeLabel> {
	fn create_edge_label(
		&self,
		v: String,
		w: String,
		name: Option<String>,
	) -> EdgeLabel;
}

impl Default for GraphOptions {
	fn default() -> Self {
		Self {
			directed: true,
			multigraph: false,
			compound: false,
		}
	}
}

impl<F, T> NodeLabelFactory<T> for F
where
	F: Fn(String) -> T,
{
	fn create_node_label(&self, v: String) -> T {
		self(v)
	}
}

impl<F, T> EdgeLabelFactory<T> for F
where
	F: Fn(String, String, Option<String>) -> T,
{
	fn create_edge_label(
		&self,
		v: String,
		w: String,
		name: Option<String>,
	) -> T {
		self(v, w, name)
	}
}
