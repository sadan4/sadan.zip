//! End-to-end smoke tests for the layout pipeline. These don't aim to match
//! the JS dagre output exactly — they verify that layout terminates, assigns
//! consistent ranks/orders/coords, and produces a sensible bounding box.

use dagre::graph::GraphOpts;
use dagre::types::{EdgeLabel, GraphLabel, NodeLabel};
use dagre::{Graph, Ranker};

fn make_node(width: f64, height: f64) -> NodeLabel {
    NodeLabel {
        width,
        height,
        ..Default::default()
    }
}

fn make_edge() -> EdgeLabel {
    EdgeLabel::default_layout()
}

fn make_graph() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
    // dagre internally builds a multigraph; acyclic reversal uses named edges,
    // and normalize inserts named dummy edges. So users must use a multigraph.
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
        Graph::with_opts(GraphOpts::directed().multigraph());
    g.set_graph(GraphLabel::defaults());
    g
}

#[test]
fn empty_graph_layouts_without_panic() {
    let mut g = make_graph();
    dagre::layout(&mut g);
}

#[test]
fn single_node_gets_coords() {
    let mut g = make_graph();
    g.set_node("a", make_node(50.0, 30.0));
    dagre::layout(&mut g);
    let n = g.node("a").unwrap();
    assert!(n.x.is_some());
    assert!(n.y.is_some());
    assert!(n.rank.is_some());
}

#[test]
fn three_node_chain_assigns_increasing_ranks_and_ys() {
    let mut g = make_graph();
    if let Some(gl) = g.graph_mut() {
        gl.ranker = Some(Ranker::LongestPath);
    }
    g.set_node("a", make_node(50.0, 30.0));
    g.set_node("b", make_node(50.0, 30.0));
    g.set_node("c", make_node(50.0, 30.0));
    g.set_edge("a", "b", make_edge());
    g.set_edge("b", "c", make_edge());

    dagre::layout(&mut g);

    let ya = g.node("a").unwrap().y.unwrap();
    let yb = g.node("b").unwrap().y.unwrap();
    let yc = g.node("c").unwrap().y.unwrap();
    assert!(ya < yb, "a.y ({}) should be < b.y ({})", ya, yb);
    assert!(yb < yc, "b.y ({}) should be < c.y ({})", yb, yc);

    let ra = g.node("a").unwrap().rank.unwrap();
    let rb = g.node("b").unwrap().rank.unwrap();
    let rc = g.node("c").unwrap().rank.unwrap();
    // Ranks are doubled by makeSpaceForEdgeLabels — only require monotonic.
    assert!(rb > ra && rc > rb, "ranks must increase: {} {} {}", ra, rb, rc);
}

#[test]
fn diamond_has_both_branches_at_same_rank() {
    let mut g = make_graph();
    if let Some(gl) = g.graph_mut() {
        gl.ranker = Some(Ranker::LongestPath);
    }
    g.set_node("a", make_node(40.0, 20.0));
    g.set_node("b", make_node(40.0, 20.0));
    g.set_node("c", make_node(40.0, 20.0));
    g.set_node("d", make_node(40.0, 20.0));
    g.set_edge("a", "b", make_edge());
    g.set_edge("a", "c", make_edge());
    g.set_edge("b", "d", make_edge());
    g.set_edge("c", "d", make_edge());

    dagre::layout(&mut g);

    let rb = g.node("b").unwrap().rank.unwrap();
    let rc = g.node("c").unwrap().rank.unwrap();
    assert_eq!(rb, rc, "b and c should be in the same rank");

    let xb = g.node("b").unwrap().x.unwrap();
    let xc = g.node("c").unwrap().x.unwrap();
    assert!((xb - xc).abs() > 1.0, "b and c should be horizontally separated, got xb={} xc={}", xb, xc);
}

#[test]
fn cycle_is_broken_and_restored() {
    let mut g = make_graph();
    if let Some(gl) = g.graph_mut() {
        gl.ranker = Some(Ranker::LongestPath);
    }
    g.set_node("a", make_node(20.0, 20.0));
    g.set_node("b", make_node(20.0, 20.0));
    g.set_node("c", make_node(20.0, 20.0));
    g.set_edge("a", "b", make_edge());
    g.set_edge("b", "c", make_edge());
    g.set_edge("c", "a", make_edge());

    dagre::layout(&mut g);

    assert_eq!(g.edge_count(), 3, "all three edges should remain after acyclic undo");
    // At least one edge should be marked as reversed during run (then unreversed).
    // After undo, no edge should be marked reversed.
    for e in g.edges() {
        let l = g.edge_obj(&e).unwrap();
        assert!(!l.reversed, "edge {:?} still marked reversed after undo", e);
    }
}

#[test]
fn bounding_box_is_set() {
    let mut g = make_graph();
    if let Some(gl) = g.graph_mut() {
        gl.ranker = Some(Ranker::LongestPath);
        gl.marginx = Some(10.0);
        gl.marginy = Some(10.0);
    }
    g.set_node("a", make_node(60.0, 40.0));
    g.set_node("b", make_node(60.0, 40.0));
    g.set_edge("a", "b", make_edge());
    dagre::layout(&mut g);
    let gl = g.graph().unwrap();
    let w = gl.width.unwrap();
    let h = gl.height.unwrap();
    assert!(w > 0.0 && h > 0.0, "graph should have positive bounding box, got {}x{}", w, h);
}
