//! Port of test/rank/util-test.ts.

use dagre::graph::{Graph, GraphOpts};
use dagre::rank::util_rank;
use dagre::types::{EdgeLabel, GraphLabel, NodeLabel};
use dagre::util;

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::with_opts(GraphOpts::directed());
    g.set_default_edge_label(|_| EdgeLabel {
        minlen: 1,
        ..Default::default()
    });
    g
}

#[test]
fn longest_path_single_node() {
    let mut g = mk();
    g.set_node("a", NodeLabel::default());
    util_rank::longest_path(&mut g);
    util::normalize_ranks(&mut g);
    assert_eq!(g.node("a").unwrap().rank, Some(0));
}

#[test]
fn longest_path_unconnected_nodes() {
    let mut g = mk();
    g.set_node("a", NodeLabel::default());
    g.set_node("b", NodeLabel::default());
    util_rank::longest_path(&mut g);
    util::normalize_ranks(&mut g);
    assert_eq!(g.node("a").unwrap().rank, Some(0));
    assert_eq!(g.node("b").unwrap().rank, Some(0));
}

#[test]
fn longest_path_connected_nodes() {
    let mut g = mk();
    g.set_edge_default("a", "b");
    util_rank::longest_path(&mut g);
    util::normalize_ranks(&mut g);
    assert_eq!(g.node("a").unwrap().rank, Some(0));
    assert_eq!(g.node("b").unwrap().rank, Some(1));
}

#[test]
fn longest_path_diamond() {
    let mut g = mk();
    g.set_path(&["a", "b", "d"]);
    g.set_path(&["a", "c", "d"]);
    util_rank::longest_path(&mut g);
    util::normalize_ranks(&mut g);
    assert_eq!(g.node("a").unwrap().rank, Some(0));
    assert_eq!(g.node("b").unwrap().rank, Some(1));
    assert_eq!(g.node("c").unwrap().rank, Some(1));
    assert_eq!(g.node("d").unwrap().rank, Some(2));
}

#[test]
fn longest_path_uses_minlen() {
    let mut g = mk();
    g.set_path(&["a", "b", "d"]);
    g.set_edge_default("a", "c");
    g.set_edge(
        "c",
        "d",
        EdgeLabel {
            minlen: 2,
            ..Default::default()
        },
    );
    util_rank::longest_path(&mut g);
    util::normalize_ranks(&mut g);
    assert_eq!(g.node("a").unwrap().rank, Some(0));
    // longest path biases towards lowest rank — b sits at the bottom of a path.
    assert_eq!(g.node("b").unwrap().rank, Some(2));
    assert_eq!(g.node("c").unwrap().rank, Some(1));
    assert_eq!(g.node("d").unwrap().rank, Some(3));
}
