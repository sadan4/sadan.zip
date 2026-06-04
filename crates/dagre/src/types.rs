//! Label types matching `lib/types.ts`.
//!
//! In the JS code labels are plain objects with many optional fields. We
//! mirror that with Rust structs where every situational field is `Option`.

use crate::graph::{Edge, NodeId};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Point {
	pub x: f64,
	pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Dummy {
	Edge,
	Border,
	EdgeLabel,
	EdgeProxy,
	SelfEdge,
	Root,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BorderType {
	BorderLeft,
	BorderRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LabelPos {
	L,
	C,
	R,
}

impl LabelPos {
	#[expect(clippy::should_implement_trait)]
	pub fn from_str(s: &str) -> Option<Self> {
		match s.to_lowercase().as_str() {
			"l" => Some(Self::L),
			"c" => Some(Self::C),
			"r" => Some(Self::R),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RankDir {
	TB,
	BT,
	LR,
	RL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Align {
	UL,
	UR,
	DL,
	DR,
}

impl Align {
	pub const fn to_str(&self) -> &'static str {
		match self {
			Self::UL => "ul",
			Self::UR => "ur",
			Self::DL => "dl",
			Self::DR => "dr",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RankAlign {
	Top,
	Center,
	Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Ranker {
	NetworkSimplex,
	TightTree,
	LongestPath,
	None,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeLabel {
	/// Input width of the node, used for spacing during layout.
	pub width: f64,
	/// Input height of the node, used for spacing during layout.
	pub height: f64,
	/// Output x-coordinate of the node center, set by the position phase.
	pub x: Option<f64>,
	/// Output y-coordinate of the node center, set by the position phase.
	pub y: Option<f64>,
	/// Layer index assigned by the ranking phase; lower ranks are closer to the source.
	pub rank: Option<i32>,
	/// Position within the rank, set by the ordering phase to minimize edge crossings.
	pub order: Option<usize>,

	/// Marks synthetic nodes inserted by the layout (edge splits, borders, etc.).
	pub dummy: Option<Dummy>,
	/// For border dummy nodes, distinguishes left vs right border segments.
	pub border_type: Option<BorderType>,
	/// Compound graphs: node ID of the top border anchor.
	pub border_top: Option<SmolStr>,
	/// Compound graphs: node ID of the bottom border anchor.
	pub border_bottom: Option<SmolStr>,
	/// Compound graphs: left border node IDs, indexed by rank.
	pub border_left: Option<Vec<SmolStr>>,
	/// Compound graphs: right border node IDs, indexed by rank.
	pub border_right: Option<Vec<SmolStr>>,
	/// Compound graphs: smallest rank a cluster subgraph may occupy.
	pub min_rank: Option<i32>,
	/// Compound graphs: largest rank a cluster subgraph may occupy.
	pub max_rank: Option<i32>,

	/// Where the node's label sits relative to the node (L/C/R).
	pub labelpos: Option<LabelPos>,

	/// Set for "edge" dummy nodes: original edge object.
	pub edge_obj: Option<Edge>,
	/// Set for "edge" dummy nodes: original edge label (small, so we box).
	pub edge_label: Option<Box<EdgeLabel>>,

	/// Internal: self-edges stashed off a node while ranking/ordering.
	pub self_edges: Option<Vec<SelfEdgeStash>>,

	/// Used internally by greedy-fas / order to attach algorithm-specific
	/// numeric state to a node when its label type is fixed.
	pub e: Option<f64>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SelfEdgeStash {
	pub e: Edge,
	pub label: EdgeLabel,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeLabel {
	/// Computed spline waypoints for the edge, populated by the position phase.
	pub points: Option<Vec<Point>>,
	/// Width of the edge's label box, used to reserve space along the edge.
	pub width: f64,
	/// Height of the edge's label box, used to reserve space along the edge.
	pub height: f64,
	/// Minimum number of ranks the edge must span; enforced by ranking.
	pub minlen: i32,
	/// Importance weight; higher weights bias ranking and crossing-minimization.
	pub weight: f64,
	/// Where the label sits along the edge (L/C/R).
	pub labelpos: Option<LabelPos>,
	/// Distance to offset the label from the edge path.
	pub labeloffset: f64,
	/// Rank at which to place the edge-label dummy node, for long labelled edges.
	pub label_rank: Option<i32>,
	/// Output x-coordinate of the edge label, set by the position phase.
	pub x: Option<f64>,
	/// Output y-coordinate of the edge label, set by the position phase.
	pub y: Option<f64>,

	/// True if the edge was reversed by the acyclic phase to break a cycle.
	pub reversed: bool,
	/// Original edge name preserved when reversed, so the acyclic phase can undo the flip.
	pub forward_name: Option<NodeId>,
	/// True if the edge is a self-loop.
	pub self_edge: bool,
	/// True if the edge was inserted to enforce compound-graph nesting.
	pub nesting_edge: bool,

	/// Network-simplex tree-edge state: cut value; negatives mark edges that should leave the tree.
	pub cutvalue: Option<f64>,
	/// Network-simplex tree state: DFS post-order index ("lim") of the head node.
	pub lim: Option<i32>,
	/// Network-simplex tree state: lowest DFS index in the subtree rooted at the head node.
	pub low: Option<i32>,
	/// Network-simplex tree state: parent node in the spanning tree.
	pub parent: Option<NodeId>,
}

impl EdgeLabel {
	pub fn default_layout() -> Self {
		Self {
			minlen: 1,
			weight: 1.0,
			width: 0.0,
			height: 0.0,
			labeloffset: 10.0,
			labelpos: Some(LabelPos::R),
			..Default::default()
		}
	}
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GraphLabel {
	/// Output total width of the laid-out graph.
	pub width: Option<f64>,
	/// Output total height of the laid-out graph.
	pub height: Option<f64>,
	/// True when the graph contains nested subgraphs (clusters).
	pub compound: bool,

	/// Direction of rank flow: TB, BT, LR, or RL.
	pub rankdir: Option<RankDir>,
	/// Alignment of nodes within a rank (UL/UR/DL/DR).
	pub align: Option<Align>,
	/// Minimum separation between adjacent nodes within a rank.
	pub nodesep: Option<f64>,
	/// Minimum separation between adjacent edges.
	pub edgesep: Option<f64>,
	/// Separation between adjacent ranks.
	pub ranksep: Option<f64>,
	/// Horizontal margin around the entire laid-out graph.
	pub marginx: Option<f64>,
	/// Vertical margin around the entire laid-out graph.
	pub marginy: Option<f64>,

	/// Cycle-breaking strategy; "greedy" selects greedy-FAS, otherwise a DFS-based pass is used.
	pub acyclicer: Option<String>,
	/// Ranking algorithm to use.
	pub ranker: Option<Ranker>,
	/// How ranks are aligned vertically within their layer (Top/Center/Bottom).
	pub rank_align: Option<RankAlign>,

	/// Compound graphs: root node ID of the nesting hierarchy.
	pub nesting_root: Option<NodeId>,
	/// Compound graphs: divisor controlling rank spacing across nesting levels.
	pub node_rank_factor: Option<f64>,
	/// Starting nodes of dummy chains inserted by normalize, used to undo normalization.
	pub dummy_chains: Option<Vec<NodeId>>,

	/// Largest rank index in the graph after ranking; computed by the ranking phase.
	pub max_rank: Option<i32>,
}

impl GraphLabel {
	pub fn defaults() -> Self {
		Self {
			ranksep: Some(50.0),
			edgesep: Some(20.0),
			nodesep: Some(50.0),
			rankdir: Some(RankDir::TB),
			rank_align: Some(RankAlign::Center),
			..Default::default()
		}
	}
}

/// Type alias for the standard layout graph.
pub type LayoutGraph = crate::graph::Graph<GraphLabel, NodeLabel, EdgeLabel>;
