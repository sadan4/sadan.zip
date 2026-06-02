//! Port of test/greedy-fas-test.ts.

use dagre::graph::{alg, Edge, Graph, GraphOpts};
use dagre::greedy_fas::greedy_fas;
use dagre::types::{EdgeLabel, GraphLabel, NodeLabel};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::new();
    g.set_default_edge_label(|_| EdgeLabel {
        weight: 1.0,
        minlen: 1,
        ..Default::default()
    });
    g
}

fn mk_mg() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
    let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
        Graph::with_opts(GraphOpts::directed().multigraph());
    g.set_default_edge_label(|_| EdgeLabel {
        weight: 1.0,
        minlen: 1,
        ..Default::default()
    });
    g
}

fn default_weight_fn(_e: &Edge) -> f64 {
    1.0
}

fn check_fas(graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>, fas: &[Edge]) {
    let n = graph.node_count() as i64;
    let m = graph.edge_count() as i64;
    let mut g = clone_graph(graph);
    for e in fas {
        g.remove_edge_obj(e);
    }
    let cycles = alg::find_cycles(&g);
    assert!(cycles.is_empty(), "still cyclic: {:?}", cycles);
    let bound = (m / 2) - (n / 6);
    assert!(
        fas.len() as i64 <= bound,
        "FAS size {} > bound {} (m={}, n={})",
        fas.len(),
        bound,
        m,
        n
    );
}

fn clone_graph(g: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
    let mut out: Graph<GraphLabel, NodeLabel, EdgeLabel> =
        Graph::with_opts(GraphOpts::directed().multigraph());
    for v in g.nodes() {
        out.set_node(v.clone(), g.node(&v).cloned().unwrap_or_default());
    }
    for e in g.edges() {
        out.set_edge_named(
            e.v.clone(),
            e.w.clone(),
            g.edge_obj(&e).cloned().unwrap_or_default(),
            e.name.clone(),
        );
    }
    out
}

#[test]
fn empty_set_for_empty_graph() {
    let g = mk();
    let fas = greedy_fas(&g, default_weight_fn);
    assert!(fas.is_empty());
}

#[test]
fn empty_set_for_single_node() {
    let mut g = mk();
    g.set_node("a", NodeLabel::default());
    assert!(greedy_fas(&g, default_weight_fn).is_empty());
}

#[test]
fn empty_set_for_acyclic_graph() {
    let mut g = mk();
    g.set_edge_default("a", "b");
    g.set_edge_default("b", "c");
    g.set_edge_default("b", "d");
    g.set_edge_default("a", "e");
    assert!(greedy_fas(&g, default_weight_fn).is_empty());
}

#[test]
fn single_edge_simple_cycle() {
    let mut g = mk();
    g.set_edge_default("a", "b");
    g.set_edge_default("b", "a");
    let fas = greedy_fas(&g, default_weight_fn);
    check_fas(&g, &fas);
}

#[test]
fn single_edge_in_4_node_cycle() {
    let mut g = mk();
    g.set_edge_default("n1", "n2");
    g.set_path(&["n2", "n3", "n4", "n5", "n2"]);
    g.set_edge_default("n3", "n5");
    g.set_edge_default("n4", "n2");
    g.set_edge_default("n4", "n6");
    let fas = greedy_fas(&g, default_weight_fn);
    check_fas(&g, &fas);
}

#[test]
fn two_edges_for_two_4_cycles() {
    let mut g = mk();
    g.set_edge_default("n1", "n2");
    g.set_path(&["n2", "n3", "n4", "n5", "n2"]);
    g.set_edge_default("n3", "n5");
    g.set_edge_default("n4", "n2");
    g.set_edge_default("n4", "n6");
    g.set_path(&["n6", "n7", "n8", "n9", "n6"]);
    g.set_edge_default("n7", "n9");
    g.set_edge_default("n8", "n6");
    g.set_edge_default("n8", "n10");
    let fas = greedy_fas(&g, default_weight_fn);
    check_fas(&g, &fas);
}

#[test]
fn works_with_weighted_edges() {
    let mut g1 = mk();
    g1.set_edge(
        "n1",
        "n2",
        EdgeLabel {
            weight: 2.0,
            minlen: 1,
            ..Default::default()
        },
    );
    g1.set_edge(
        "n2",
        "n1",
        EdgeLabel {
            weight: 1.0,
            minlen: 1,
            ..Default::default()
        },
    );
    let g1_ref = &g1;
    let wf = |e: &Edge| g1_ref.edge_obj(e).map(|l| l.weight).unwrap_or(1.0);
    let fas = greedy_fas(&g1, wf);
    assert_eq!(fas.len(), 1);
    assert_eq!((&fas[0].v, &fas[0].w), (&"n2".into(), &"n1".into()));

    let mut g2 = mk();
    g2.set_edge(
        "n1",
        "n2",
        EdgeLabel {
            weight: 1.0,
            minlen: 1,
            ..Default::default()
        },
    );
    g2.set_edge(
        "n2",
        "n1",
        EdgeLabel {
            weight: 2.0,
            minlen: 1,
            ..Default::default()
        },
    );
    let g2_ref = &g2;
    let wf2 = |e: &Edge| g2_ref.edge_obj(e).map(|l| l.weight).unwrap_or(1.0);
    let fas2 = greedy_fas(&g2, wf2);
    assert_eq!(fas2.len(), 1);
    assert_eq!((&fas2[0].v, &fas2[0].w), (&"n1".into(), &"n2".into()));
}

#[test]
fn works_for_multigraphs() {
    let mut g = mk_mg();
    g.set_edge_named(
        "a",
        "b",
        EdgeLabel {
            weight: 5.0,
            ..Default::default()
        },
        Some("foo".into()),
    );
    g.set_edge_named(
        "b",
        "a",
        EdgeLabel {
            weight: 2.0,
            ..Default::default()
        },
        Some("bar".into()),
    );
    g.set_edge_named(
        "b",
        "a",
        EdgeLabel {
            weight: 2.0,
            ..Default::default()
        },
        Some("baz".into()),
    );
    let g_ref = &g;
    let wf = |e: &Edge| g_ref.edge_obj(e).map(|l| l.weight).unwrap_or(1.0);
    let mut fas = greedy_fas(&g, wf);
    fas.sort_by(|a, b| a.name.cmp(&b.name));
    // Expect "bar" + "baz" reversed (b -> a).
    assert_eq!(fas.len(), 2);
    assert_eq!(
        fas.iter()
            .map(|e| (e.v.clone(), e.w.clone(), e.name.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("b".into(), "a".into(), Some("bar".into())),
            ("b".into(), "a".into(), Some("baz".into())),
        ]
    );
}
