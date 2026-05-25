use dagre_graphlib::Edge;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RankDir {
	TB,
	BT,
	LR,
	RL,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Align {
	UL,
	UR,
	DL,
	DR,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Acyclicer {
	Greedy,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ranker {
	NetworkSimplex,
	TightTree,
	LongestPath,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RankAlign {
	Top,
	Center,
	Bottom,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Dummy {
	Edge,
	Border,
	EdgeLabel,
	EdgeProxy,
	SelfEdge,
	Root,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BorderType {
	BorderLeft,
	BorderRight,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LabelPos {
	Left,
	Center,
	Right,
}

// port start

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Point {
	pub x: i32,
	pub y: i32,
}

#[derive(Clone, Default)]
pub struct NodeLabel {
	pub width: u32,
	pub height: u32,
	pub x: Option<i32>,
	pub y: Option<i32>,
	pub rank: Option<i32>,
	pub order: Option<u32>,
	pub e: Option<u32>,
	pub dummy: Option<Dummy>,
	pub border_type: Option<BorderType>,
	pub border_top: Option<String>,
	pub border_bottom: Option<String>,
	pub border_left: Option<Vec<String>>,
	pub border_right: Option<Vec<String>>,
	pub min_rank: Option<u32>,
	pub max_rank: Option<u32>,
	pub label: Option<String>,
	pub labelpos: Option<LabelPos>,
	pub class: Option<String>,
	pub padding: Option<u32>,
	pub padding_x: Option<u32>,
	pub padding_y: Option<u32>,
	pub rx: Option<u32>,
	pub ry: Option<u32>,
	pub shape: Option<String>,
	pub edge_label: Option<EdgeLabel>,
	pub edge_obj: Option<Edge>,
}

#[derive(Clone, Default)]
pub struct EdgeLabel {
	pub points: Option<Vec<Point>>,
	pub width: Option<u32>,
	pub height: Option<u32>,
	pub minlen: Option<u32>,
	pub weight: Option<i32>,
	pub labelpos: Option<LabelPos>,
	pub labeloffset: Option<u32>,
	pub label_rank: Option<u32>,
	pub x: Option<u32>,
	pub y: Option<u32>,
	pub e: Option<u32>,
	pub reserved: Option<bool>,
	pub forward_name: Option<String>,
	pub self_edge: Option<bool>,
	pub nesting_edge: Option<bool>,
	pub cutvalue: Option<u32>,
	pub lim: Option<u32>,
	pub low: Option<u32>,
	pub parent: Option<String>,
	pub edge_label: Option<Box<EdgeLabel>>,
	pub edge_obj: Option<Edge>,
}

#[derive(Default, Clone)]
pub struct GraphLabel {
	pub width: Option<u32>,
	pub height: Option<u32>,
	pub compound: Option<bool>,
	pub rankdir: Option<RankDir>,
	pub align: Option<Align>,
	pub nodesep: Option<u32>,
	pub edgesep: Option<u32>,
	pub ranksep: Option<u32>,
	pub marginx: Option<u32>,
	pub marginy: Option<u32>,
	pub acyclicer: Option<Acyclicer>,
	pub ranker: Option<Ranker>,
	pub rankalign: Option<RankAlign>,
	pub nesting_root: Option<String>,
	pub node_rank_factor: Option<u32>,
	pub dummy_chains: Option<Vec<String>>,
}
