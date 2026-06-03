//! Port of src/bench.ts. Runs each ranker (longest-path, tight-tree,
//! network-simplex) plus the full layout pipeline against a small Gansner
//! graph and a larger 25-group × 27-node synthetic graph that exercises
//! cross-component complexity.

use criterion::{criterion_group, criterion_main, Criterion};
use dagre::{
	LayoutGraph, graph::{Graph, GraphOpts}, rank, types::{EdgeLabel, GraphLabel, NodeLabel, Ranker}
};

fn make_graph() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g: Graph<GraphLabel, NodeLabel, EdgeLabel> = Graph::with_opts(
		GraphOpts::directed()
			.multigraph()
			.compound(),
	);
	g.set_graph(GraphLabel::defaults());
	g.set_default_node_label(|_| NodeLabel {
		width: 1.0,
		height: 1.0,
		..Default::default()
	});
	g.set_default_edge_label(|_| EdgeLabel {
		minlen: 1,
		weight: 1.0,
		..Default::default()
	});
	g
}

/// Small graph from Gansner et al. — same one the JS bench uses as a
/// baseline.
fn small_graph() -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut g = make_graph();
	g.set_path(&["a", "b", "c", "d", "h"]);
	g.set_path(&["a", "e", "g", "h"]);
	g.set_path(&["a", "f", "g"]);
	g
}

/// Larger graph (~675 nodes) with intra-group backbone + branches and
/// cross-group connections. We use a deterministic LCG-like counter to
/// match the JS bench's intent of "randomized" cross-edges while keeping
/// the benchmark reproducible (true `Math.random()` in JS doesn't seed).
fn large_graph() -> LayoutGraph {
	let mut g = make_graph();
	let num_groups: usize = 25;
	let nodes_per_group: usize = 27;

	for group in 0..num_groups {
		let mut group_nodes: Vec<String> = Vec::with_capacity(nodes_per_group);
		for node in 0..nodes_per_group {
			let id = format!("g{group}_n{node}");
			g.set_node(
				id.clone(),
				NodeLabel {
					width: 1.0,
					height: 1.0,
					..Default::default()
				},
			);
			group_nodes.push(id);
		}

		// Backbone: every 3rd hop.
		let mut i = 0;
		while i + 3 < group_nodes.len() {
			g.set_edge(
				group_nodes[i].clone(),
				group_nodes[i + 3].clone(),
				EdgeLabel {
					minlen: 1,
					weight: 1.0,
					..Default::default()
				},
			);
			i += 3;
		}

		// Branching: a small fan-out from every 4th node.
		let mut i = 1;
		while i + 2 < group_nodes.len() {
			g.set_edge(
				group_nodes[i - 1].clone(),
				group_nodes[i].clone(),
				EdgeLabel {
					minlen: 1,
					weight: 1.0,
					..Default::default()
				},
			);
			g.set_edge(
				group_nodes[i].clone(),
				group_nodes[i + 1].clone(),
				EdgeLabel {
					minlen: 1,
					weight: 1.0,
					..Default::default()
				},
			);
			g.set_edge(
				group_nodes[i].clone(),
				group_nodes[i + 2].clone(),
				EdgeLabel {
					minlen: 1,
					weight: 1.0,
					..Default::default()
				},
			);
			i += 4;
		}
	}

	// Inter-group connections — ranking-conflict generators.
	for group in 0..num_groups - 1 {
		for offset in 0..3 {
			let from = format!("g{}_n{}", group, 5 + offset * 7);
			let to = format!("g{}_n{}", group + 1, 2 + offset * 8);
			if g.has_node(&from) && g.has_node(&to) {
				g.set_edge(
					from,
					to,
					EdgeLabel {
						minlen: 1,
						weight: 1.0,
						..Default::default()
					},
				);
			}
		}
		if group + 2 < num_groups {
			let from = format!("g{group}_n10");
			let to = format!("g{}_n3", group + 2);
			if g.has_node(&from) && g.has_node(&to) {
				g.set_edge(
					from,
					to,
					EdgeLabel {
						minlen: 1,
						weight: 1.0,
						..Default::default()
					},
				);
			}
		}
	}

	// Deterministic pseudo-random cross-edges (a simple LCG so reruns are
	// comparable).
	let mut state: u64 = 0xdead_beef;
	let next = |s: &mut u64| -> u64 {
		*s = s
			.wrapping_mul(6_364_136_223_846_793_005)
			.wrapping_add(1_442_695_040_888_963_407);
		*s >> 33
	};
	for _ in 0..(num_groups * 2) {
		let from_group = (next(&mut state) as usize) % (num_groups - 1);
		let span = (next(&mut state) as usize) % 3 + 1;
		let to_group = (from_group + span).min(num_groups - 1);
		let from_node = (next(&mut state) as usize) % nodes_per_group;
		let to_node = (next(&mut state) as usize) % nodes_per_group;
		let from = format!("g{from_group}_n{from_node}");
		let to = format!("g{to_group}_n{to_node}");
		if g.has_node(&from) && g.has_node(&to) && !g.has_edge(&from, &to) {
			g.set_edge(
				from,
				to,
				EdgeLabel {
					minlen: 1,
					weight: 1.0,
					..Default::default()
				},
			);
		}
	}
	g
}

fn bench_ranker(
	c: &mut Criterion,
	name: &str,
	base: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	ranker: Ranker,
) {
	c.bench_function(name, |b| {
		b.iter_batched(
			|| {
				let mut g = clone_graph(base);
				if let Some(gl) = g.graph_mut() {
					gl.ranker = Some(ranker);
				}
				g
			},
			|mut g| rank::rank(&mut g),
			criterion::BatchSize::SmallInput,
		);
	});
}

fn bench_layout(
	c: &mut Criterion,
	name: &str,
	base: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) {
	c.bench_function(name, |b| {
		b.iter_batched(
			|| clone_graph(base),
			|mut g| dagre::layout(&mut g),
			criterion::BatchSize::SmallInput,
		);
	});
}

/// `Graph` doesn't implement `Clone` (Box<dyn Fn> default factories), so
/// we rebuild a fresh copy from its node/edge set for each iteration.
fn clone_graph(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	let mut out = make_graph();
	if let Some(gl) = g.graph() {
		out.set_graph(gl.clone());
	}
	for v in g.nodes() {
		if let Some(n) = g.node(&v) {
			out.set_node(v.clone(), n.clone());
		}
	}
	for e in g.edges() {
		if let Some(l) = g.edge_obj(&e) {
			out.set_edge_named(
				e.v.clone(),
				e.w.clone(),
				l.clone(),
				e.name.clone(),
			);
		}
	}
	out
}

fn small_benches(c: &mut Criterion) {
	let g = small_graph();
	bench_ranker(c, "longest-path ranker (small)", &g, Ranker::LongestPath);
	bench_ranker(c, "tight-tree ranker (small)", &g, Ranker::TightTree);
	bench_ranker(
		c,
		"network-simplex ranker (small)",
		&g,
		Ranker::NetworkSimplex,
	);
	bench_layout(c, "layout (small)", &g);
}

fn large_benches(c: &mut Criterion) {
	let g = large_graph();
	eprintln!(
		"Large graph: {} nodes, {} edges",
		g.node_count(),
		g.edge_count()
	);
	bench_ranker(c, "longest-path ranker (large)", &g, Ranker::LongestPath);
	bench_ranker(c, "tight-tree ranker (large)", &g, Ranker::TightTree);
	bench_ranker(
		c,
		"network-simplex ranker (large)",
		&g,
		Ranker::NetworkSimplex,
	);
	bench_layout(c, "layout (large)", &g);
}

criterion_group!(benches, small_benches, large_benches);
criterion_main!(benches);
