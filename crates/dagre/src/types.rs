//! Label types matching `lib/types.ts`.
//!
//! In the JS code labels are plain objects with many optional fields. We
//! mirror that with Rust structs where every situational field is `Option`.

use crate::graph::{Edge, NodeId};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
	pub width: f64,
	pub height: f64,
	pub x: Option<f64>,
	pub y: Option<f64>,
	pub rank: Option<i32>,
	pub order: Option<usize>,

	pub dummy: Option<Dummy>,
	pub border_type: Option<BorderType>,
	pub border_top: Option<NodeId>,
	pub border_bottom: Option<NodeId>,
	pub border_left: Option<Vec<NodeId>>,
	pub border_right: Option<Vec<NodeId>>,
	pub min_rank: Option<i32>,
	pub max_rank: Option<i32>,

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
	pub points: Option<Vec<Point>>,
	pub width: f64,
	pub height: f64,
	pub minlen: i32,
	pub weight: f64,
	pub labelpos: Option<LabelPos>,
	pub labeloffset: f64,
	pub label_rank: Option<i32>,
	pub x: Option<f64>,
	pub y: Option<f64>,

	pub reversed: bool,
	pub forward_name: Option<NodeId>,
	pub self_edge: bool,
	pub nesting_edge: bool,

	/// Network-simplex tree-edge state.
	pub cutvalue: Option<f64>,
	pub lim: Option<i32>,
	pub low: Option<i32>,
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
	pub width: Option<f64>,
	pub height: Option<f64>,
	pub compound: bool,

	pub rankdir: Option<RankDir>,
	pub align: Option<Align>,
	pub nodesep: Option<f64>,
	pub edgesep: Option<f64>,
	pub ranksep: Option<f64>,
	pub marginx: Option<f64>,
	pub marginy: Option<f64>,

	pub acyclicer: Option<String>,
	pub ranker: Option<Ranker>,
	pub rank_align: Option<RankAlign>,

	pub nesting_root: Option<NodeId>,
	pub node_rank_factor: Option<f64>,
	pub dummy_chains: Option<Vec<NodeId>>,

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
