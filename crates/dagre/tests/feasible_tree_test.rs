//! Port of test/rank/feasible-tree-test.ts.

use dagre::graph::Graph;
use dagre::rank::feasible_tree;
use dagre::types::{EdgeLabel, GraphLabel, NodeLabel};

#[test]
fn trivial_two_node_graph() {
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
    g.set_node(
        "a",
        NodeLabel {
            rank: Some(0),
            ..Default::default()
        },
    );
    g.set_node(
        "b",
        NodeLabel {
            rank: Some(1),
            ..Default::default()
        },
    );
    g.set_edge(
        "a",
        "b",
        EdgeLabel {
            minlen: 1,
            ..Default::default()
        },
    );

    let tree = feasible_tree::build(&mut g);
    assert_eq!(
        g.node("b").unwrap().rank.unwrap(),
        g.node("a").unwrap().rank.unwrap() + 1
    );
    assert_eq!(tree.neighbors("a").unwrap(), vec!["b".to_string()]);
}

#[test]
fn shortens_slack_by_pulling_up() {
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
    g.set_node(
        "a",
        NodeLabel {
            rank: Some(0),
            ..Default::default()
        },
    );
    g.set_node(
        "b",
        NodeLabel {
            rank: Some(1),
            ..Default::default()
        },
    );
    g.set_node(
        "c",
        NodeLabel {
            rank: Some(2),
            ..Default::default()
        },
    );
    g.set_node(
        "d",
        NodeLabel {
            rank: Some(2),
            ..Default::default()
        },
    );
    let el = || EdgeLabel {
        minlen: 1,
        ..Default::default()
    };
    g.set_edge("a", "b", el());
    g.set_edge("b", "c", el());
    g.set_edge("a", "d", el());

    let tree = feasible_tree::build(&mut g);
    let ra = g.node("a").unwrap().rank.unwrap();
    let rb = g.node("b").unwrap().rank.unwrap();
    let rc = g.node("c").unwrap().rank.unwrap();
    let rd = g.node("d").unwrap().rank.unwrap();
    assert_eq!(rb, ra + 1);
    assert_eq!(rc, rb + 1);
    assert_eq!(rd, ra + 1);

    let mut na = tree.neighbors("a").unwrap();
    na.sort();
    assert_eq!(na, vec!["b".to_string(), "d".to_string()]);
    let mut nb = tree.neighbors("b").unwrap();
    nb.sort();
    assert_eq!(nb, vec!["a".to_string(), "c".to_string()]);
    assert_eq!(tree.neighbors("c").unwrap(), vec!["b".to_string()]);
    assert_eq!(tree.neighbors("d").unwrap(), vec!["a".to_string()]);
}

#[test]
fn shortens_slack_by_pulling_down() {
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
    g.set_node(
        "a",
        NodeLabel {
            rank: Some(2),
            ..Default::default()
        },
    );
    g.set_node(
        "b",
        NodeLabel {
            rank: Some(0),
            ..Default::default()
        },
    );
    g.set_node(
        "c",
        NodeLabel {
            rank: Some(2),
            ..Default::default()
        },
    );
    let el = || EdgeLabel {
        minlen: 1,
        ..Default::default()
    };
    g.set_edge("b", "a", el());
    g.set_edge("b", "c", el());

    let tree = feasible_tree::build(&mut g);
    let ra = g.node("a").unwrap().rank.unwrap();
    let rb = g.node("b").unwrap().rank.unwrap();
    let rc = g.node("c").unwrap().rank.unwrap();
    assert_eq!(ra, rb + 1);
    assert_eq!(rc, rb + 1);
    assert_eq!(tree.neighbors("a").unwrap(), vec!["b".to_string()]);
    let mut nb = tree.neighbors("b").unwrap();
    nb.sort();
    assert_eq!(nb, vec!["a".to_string(), "c".to_string()]);
    assert_eq!(tree.neighbors("c").unwrap(), vec!["b".to_string()]);
}
