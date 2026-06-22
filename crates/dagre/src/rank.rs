//! Rank assignment — port of `lib/rank/*.ts`.

use crate::{
	graph::{Edge, Graph, NodeId},
	types::{EdgeLabel, GraphLabel, NodeLabel, Ranker},
	util::simplify,
};

pub mod util_rank {
	use std::collections::HashSet;

	use super::{Edge, EdgeLabel, Graph, GraphLabel, NodeId, NodeLabel};

	/// Initializes ranks using longest-path DFS from sources.
	pub fn longest_path(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
		let mut visited: HashSet<NodeId> = HashSet::new();

		fn dfs(
			graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
			v: &str,
			visited: &mut HashSet<NodeId>,
		) -> i32 {
			if visited.contains(v) {
				return graph
					.node(v)
					.and_then(|n| n.rank)
					.unwrap_or(0);
			}
			visited.insert(v.into());

			let out = graph.out_edges(v).unwrap_or_default();
			let mut min_rank = i32::MAX;
			let mut had = false;
			for e in out {
				let target = e.w.clone();
				let r = dfs(graph, &target, visited);
				let minlen = graph
					.edge_obj(&e)
					.map_or(1, |l| l.minlen);
				let candidate = r - minlen;
				if candidate < min_rank {
					min_rank = candidate;
				}
				had = true;
			}
			let rank = if had { min_rank } else { 0 };
			if let Some(n) = graph.node_mut(v) {
				n.rank = Some(rank);
			}
			rank
		}

		let sources = graph.sources();
		for s in sources {
			dfs(graph, &s, &mut visited);
		}
	}

	pub fn slack(
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		e: &Edge,
	) -> i32 {
		let v_rank = graph
			.node(&e.v)
			.and_then(|n| n.rank)
			.unwrap_or(0);
		let w_rank = graph
			.node(&e.w)
			.and_then(|n| n.rank)
			.unwrap_or(0);
		let minlen = graph
			.edge_obj(e)
			.map_or(1, |l| l.minlen);
		w_rank - v_rank - minlen
	}
}

pub mod feasible_tree {
	use super::{Edge, EdgeLabel, Graph, GraphLabel, NodeLabel};
	use crate::graph::GraphOpts;

	#[derive(Debug, Clone, Default)]
	pub struct TreeNode {
		pub low: Option<i32>,
		pub lim: Option<i32>,
		pub parent: Option<crate::graph::NodeId>,
	}

	#[derive(Debug, Clone, Default)]
	pub struct TreeEdge {
		pub cutvalue: Option<f64>,
	}

	pub type Tree = Graph<(), TreeNode, TreeEdge>;

	pub fn build(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) -> Tree {
		let mut tree: Tree = Graph::with_opts(GraphOpts {
			directed: false,
			multigraph: false,
			compound: false,
		});
		let nodes = graph.nodes();
		if nodes.is_empty() {
			panic!("Graph must have at least one node");
		}
		let start = nodes[0].clone();
		let size = graph.node_count();
		tree.set_node(start, TreeNode::default());

		while tight_tree(&mut tree, graph) < size {
			let Some(edge) = find_min_slack_edge(&tree, graph) else {
				break;
			};
			let delta = if tree.has_node(&edge.v) {
				super::util_rank::slack(graph, &edge)
			} else {
				-super::util_rank::slack(graph, &edge)
			};
			for v in tree.nodes() {
				if let Some(n) = graph.node_mut(&v)
					&& let Some(r) = n.rank
				{
					n.rank = Some(r + delta);
				}
			}
		}
		tree
	}

	fn tight_tree(
		tree: &mut Tree,
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	) -> usize {
		// Repeatedly walk tree nodes and grow.
		loop {
			let before = tree.node_count();
			for v in tree.nodes() {
				let es = graph.node_edges(&v).unwrap_or_default();
				for e in es {
					let w = if v == e.v { e.w.clone() } else { e.v.clone() };
					if !tree.has_node(&w)
						&& super::util_rank::slack(graph, &e) == 0
					{
						tree.set_node(w.clone(), TreeNode::default());
						tree.set_edge(
							v.clone(),
							w.clone(),
							TreeEdge::default(),
						);
					}
				}
			}
			if tree.node_count() == before {
				break;
			}
		}
		tree.node_count()
	}

	fn find_min_slack_edge(
		tree: &Tree,
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	) -> Option<Edge> {
		let mut best: Option<(i32, Edge)> = None;
		for e in graph.edges() {
			if tree.has_node(&e.v) != tree.has_node(&e.w) {
				let s = super::util_rank::slack(graph, &e);
				if best
					.as_ref()
					.is_none_or(|(b, _)| s < *b)
				{
					best = Some((s, e));
				}
			}
		}
		best.map(|(_, e)| e)
	}
}

pub mod network_simplex {
	use super::{
		Edge,
		EdgeLabel,
		Graph,
		GraphLabel,
		NodeId,
		NodeLabel,
		feasible_tree::{Tree, TreeEdge, TreeNode},
		simplify,
		util_rank::{longest_path, slack},
	};
	use crate::graph::alg::{postorder, preorder};
	use std::{collections::HashSet, mem};

	pub fn run(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
		let mut simplified = simplify(graph);
		longest_path(&mut simplified);
		let mut t = super::feasible_tree::build(&mut simplified);
		init_low_lim(&mut t, None);
		init_cut_values(&mut t, &simplified);

		while let Some(e) = leave_edge(&t) {
			let f = enter_edge(&t, &simplified, &e);
			exchange_edges(&mut t, &mut simplified, &e, &f);
		}

		// Copy ranks back to original graph (which may be multigraph).
		let nodes = graph.nodes();
		for v in nodes {
			if let Some(r) = simplified.node(&v).and_then(|n| n.rank)
				&& let Some(n) = graph.node_mut(&v)
			{
				n.rank = Some(r);
			}
		}
	}

	pub fn init_cut_values(
		tree: &mut Tree,
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	) {
		let ns = tree.nodes();
		let mut visited = postorder(tree, &ns);
		if !visited.is_empty() {
			visited.pop();
		}
		for v in visited {
			assign_cut_value(tree, graph, &v);
		}
	}

	fn assign_cut_value(
		tree: &mut Tree,
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		child: &str,
	) {
		let Some(parent) = tree
			.node(child)
			.and_then(|n| n.parent.clone())
		else {
			return;
		};
		let cv = calc_cut_value(tree, graph, child, &parent);
		if let Some(e) = tree.edge_mut(child, &parent) {
			e.cutvalue = Some(cv);
		}
	}

	pub fn calc_cut_value(
		tree: &Tree,
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		child: &str,
		parent: &str,
	) -> f64 {
		let mut child_is_tail = true;
		let mut graph_edge = graph.edge(child, parent).cloned();
		let mut cut: f64 = 0.0;
		if graph_edge.is_none() {
			child_is_tail = false;
			graph_edge = graph.edge(parent, child).cloned();
		}
		if let Some(ge) = &graph_edge {
			cut += ge.weight;
		}
		if let Some(es) = graph.node_edges(child) {
			for e in es {
				let is_out = e.v == child;
				let other = if is_out { e.w.clone() } else { e.v.clone() };
				if other != parent {
					let points_to_head = is_out == child_is_tail;
					let other_w = graph
						.edge_obj(&e)
						.map_or(0.0, |l| l.weight);
					cut += if points_to_head { other_w } else { -other_w };
					if tree.has_edge(child, &other)
						&& let Some(other_cv) = tree
							.edge(child, &other)
							.and_then(|e| e.cutvalue)
					{
						cut +=
							if points_to_head { -other_cv } else { other_cv };
					}
				}
			}
		}
		cut
	}

	pub fn init_low_lim(tree: &mut Tree, root: Option<NodeId>) {
		let root = root.unwrap_or_else(|| tree.nodes()[0].clone());
		let mut visited = HashSet::new();
		dfs_low_lim(tree, &mut visited, 1, &root, None);
	}

	fn dfs_low_lim(
		tree: &mut Tree,
		visited: &mut HashSet<NodeId>,
		mut next_lim: i32,
		v: &str,
		parent: Option<&str>,
	) -> i32 {
		let low = next_lim;
		visited.insert(v.into());
		let neighbors = tree.neighbors(v).unwrap_or_default();
		for w in neighbors {
			if !visited.contains(&w) {
				next_lim = dfs_low_lim(tree, visited, next_lim, &w, Some(v));
			}
		}
		if let Some(n) = tree.node_mut(v) {
			n.low = Some(low);
			n.lim = Some(next_lim);
			n.parent = parent.map(NodeId::from);
		}
		next_lim + 1
	}

	pub fn leave_edge(tree: &Tree) -> Option<Edge> {
		tree.edges().into_iter().find(|e| {
			tree.edge_obj(e)
				.and_then(|l| l.cutvalue)
				.unwrap_or(0.0)
				< 0.0
		})
	}

	pub fn enter_edge(
		tree: &Tree,
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		edge: &Edge,
	) -> Edge {
		let (mut v, mut w) = (edge.v.clone(), edge.w.clone());
		if !graph.has_edge(&v, &w) {
			mem::swap(&mut v, &mut w);
		}
		let v_label = tree
			.node(&v)
			.cloned()
			.unwrap_or_default();
		let w_label = tree
			.node(&w)
			.cloned()
			.unwrap_or_default();
		let (tail_label, flip) =
			if v_label.lim.unwrap_or(0) > w_label.lim.unwrap_or(0) {
				(w_label, true)
			} else {
				(v_label, false)
			};

		let candidates: Vec<Edge> = graph
			.edges()
			.into_iter()
			.filter(|edge| {
				let v_desc = is_descendant(
					&tree
						.node(&edge.v)
						.cloned()
						.unwrap_or_default(),
					&tail_label,
				);
				let w_desc = is_descendant(
					&tree
						.node(&edge.w)
						.cloned()
						.unwrap_or_default(),
					&tail_label,
				);
				flip == v_desc && flip != w_desc
			})
			.collect();

		let mut best = candidates[0].clone();
		for c in &candidates[1..] {
			if slack(graph, c) < slack(graph, &best) {
				best = c.clone();
			}
		}
		best
	}

	fn is_descendant(v: &TreeNode, root: &TreeNode) -> bool {
		let lo = root.low.unwrap_or(0);
		let hi = root.lim.unwrap_or(0);
		let vl = v.lim.unwrap_or(0);
		lo <= vl && vl <= hi
	}

	pub fn exchange_edges(
		tree: &mut Tree,
		graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
		e: &Edge,
		f: &Edge,
	) {
		tree.remove_edge(&e.v, &e.w);
		tree.set_edge(f.v.clone(), f.w.clone(), TreeEdge::default());
		init_low_lim(tree, None);
		init_cut_values(tree, graph);
		update_ranks(tree, graph);
	}

	fn update_ranks(
		tree: &Tree,
		graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	) {
		let root = tree.nodes().into_iter().find(|v| {
			tree.node(v)
				.is_some_and(|n| n.parent.is_none())
		});
		let Some(root) = root else { return };
		let pre = preorder(tree, &[root]);
		for v in pre.into_iter().skip(1) {
			let Some(parent) = tree
				.node(&v)
				.and_then(|n| n.parent.clone())
			else {
				continue;
			};
			let (minlen, flipped) = if graph.has_edge(&v, &parent) {
				(
					graph
						.edge(&v, &parent)
						.map_or(1, |l| l.minlen),
					false,
				)
			} else {
				(
					graph
						.edge(&parent, &v)
						.map_or(1, |l| l.minlen),
					true,
				)
			};
			let parent_rank = graph
				.node(&parent)
				.and_then(|n| n.rank)
				.unwrap_or(0);
			let new_rank = parent_rank + if flipped { minlen } else { -minlen };
			if let Some(n) = graph.node_mut(&v) {
				n.rank = Some(new_rank);
			}
		}
	}
}

pub fn rank(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	if graph.node_count() == 0 {
		return;
	}
	let ranker = graph
		.graph()
		.and_then(|g| g.ranker)
		.unwrap_or(Ranker::NetworkSimplex);
	match ranker {
		Ranker::LongestPath => util_rank::longest_path(graph),
		Ranker::TightTree => {
			util_rank::longest_path(graph);
			feasible_tree::build(graph);
		}
		Ranker::NetworkSimplex => network_simplex::run(graph),
		Ranker::None => {}
	}
}
