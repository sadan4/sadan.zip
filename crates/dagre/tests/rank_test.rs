//! Port of test/rank/rank-test.ts. Tries every ranker against a graph
//! and verifies the resulting ranks respect each edge's minlen.

use dagre::{
	graph::Graph,
	rank,
	types::{EdgeLabel, GraphLabel, NodeLabel, Ranker},
};

const RANKERS: &[Ranker] = &[
	Ranker::LongestPath,
	Ranker::TightTree,
	Ranker::NetworkSimplex,
];

fn build_default_graph() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
	g.set_graph(GraphLabel::default());
	g.set_default_edge_label(|_| EdgeLabel {
		minlen: 1,
		weight: 1.0,
		..Default::default()
	});
	g.set_path(&["a", "b", "c", "d", "h"]);
	g.set_path(&["a", "e", "g", "h"]);
	g.set_path(&["a", "f", "g"]);
	g
}

#[test]
fn respects_minlen_for_each_ranker() {
	for &ranker in RANKERS {
		let mut g = build_default_graph();
		if let Some(gl) = g.graph_mut() {
			gl.ranker = Some(ranker);
		}
		rank::rank(&mut g);
		for e in g.edges() {
			let v_rank = g.node(&e.v).unwrap().rank.unwrap();
			let w_rank = g.node(&e.w).unwrap().rank.unwrap();
			let minlen = g.edge_obj(&e).unwrap().minlen;
			assert!(
				w_rank - v_rank >= minlen,
				"ranker={:?} edge {:?} -> wRank-vRank = {} < minlen {}",
				ranker,
				e,
				w_rank - v_rank,
				minlen
			);
		}
	}
}

#[test]
fn ranks_single_node_graph_for_each_ranker() {
	for &ranker in RANKERS {
		let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
		g.set_graph(GraphLabel {
			ranker: Some(ranker),
			..Default::default()
		});
		g.set_node("a", NodeLabel::default());
		rank::rank(&mut g);
		assert_eq!(g.node("a").unwrap().rank, Some(0), "ranker={:?}", ranker);
	}
}
