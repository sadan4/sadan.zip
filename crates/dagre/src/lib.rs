//! Rust port of dagre, a directed graph layout library.
//!
//! Scope: the core layout pipeline only — acyclic, rank, normalize, order,
//! position. The nesting graph / border / coordinate-system helpers from the
//! TypeScript source are intentionally omitted.

pub mod graph;
pub mod types;
pub mod util;

pub mod acyclic;
pub mod greedy_fas;
pub mod list;
pub mod normalize;
pub mod order;
pub mod position;
pub mod rank;

pub mod layout;

pub use graph::{Edge, Graph, GraphOpts};
pub use layout::layout;
pub use types::{
	Align,
	Dummy,
	EdgeLabel,
	GraphLabel,
	LabelPos,
	LayoutGraph,
	NodeLabel,
	Point,
	RankAlign,
	RankDir,
	Ranker,
};
#[cfg(test)]
mod tests {
	use crate::{LayoutGraph, layout};

	#[test]
	fn panic1_lays_out_without_panic() {
		let json = include_str!("../tests/data/panic1.json");
		let mut g: LayoutGraph = serde_json::from_str::<LayoutGraph>(json)
			.expect("failed to deserialize panic1.json");
		layout(&mut g);
	}
}
