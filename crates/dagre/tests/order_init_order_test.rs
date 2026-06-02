//! Port of test/order/init-order-test.ts.

use dagre::{
	graph::{Graph, GraphOpts},
	order::init_order,
	types::{EdgeLabel, GraphLabel, NodeLabel},
};

fn mk() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> =
		Graph::with_opts(GraphOpts::directed().compound());
	g.set_default_edge_label(|_| EdgeLabel {
		weight: 1.0,
		..Default::default()
	});
	g
}

fn set_ranked(
	g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	ranks: &[(&str, i32)],
) {
	for (v, r) in ranks {
		g.set_node(
			v.to_string(),
			NodeLabel {
				rank: Some(*r),
				..Default::default()
			},
		);
	}
}

#[test]
fn non_overlapping_orders_in_a_tree() {
	let mut g = mk();
	set_ranked(&mut g, &[("a", 0), ("b", 1), ("c", 2), ("d", 2), ("e", 1)]);
	g.set_path(&["a", "b", "c"]);
	g.set_edge_default("b", "d");
	g.set_edge_default("a", "e");
	let layering = init_order(&g);
	assert_eq!(layering[0], vec!["a".to_string()]);
	let mut l1 = layering[1].clone();
	l1.sort();
	assert_eq!(l1, vec!["b".to_string(), "e".to_string()]);
	let mut l2 = layering[2].clone();
	l2.sort();
	assert_eq!(l2, vec!["c".to_string(), "d".to_string()]);
}

#[test]
fn non_overlapping_orders_in_dag() {
	let mut g = mk();
	set_ranked(&mut g, &[("a", 0), ("b", 1), ("c", 1), ("d", 2)]);
	g.set_path(&["a", "b", "d"]);
	g.set_path(&["a", "c", "d"]);
	let layering = init_order(&g);
	assert_eq!(layering[0], vec!["a".to_string()]);
	let mut l1 = layering[1].clone();
	l1.sort();
	assert_eq!(l1, vec!["b".to_string(), "c".to_string()]);
	assert_eq!(layering[2], vec!["d".to_string()]);
}

#[test]
fn does_not_assign_order_to_subgraph_nodes() {
	let mut g = mk();
	g.set_node(
		"a",
		NodeLabel {
			rank: Some(0),
			..Default::default()
		},
	);
	g.set_node("sg1", NodeLabel::default());
	g.set_parent("a", Some("sg1"));
	let layering = init_order(&g);
	assert_eq!(layering, vec![vec!["a".to_string()]]);
}
