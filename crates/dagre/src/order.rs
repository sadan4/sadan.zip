//! Crossing-minimization order pipeline — port of `lib/order/*.ts`.

use smol_str::SmolStr;

use crate::{
	graph::{Graph, GraphOpts, NodeId},
	types::{EdgeLabel, GraphLabel, NodeLabel},
	util,
};
use std::{
	cmp,
	collections::{HashMap, HashSet},
	mem,
};

#[derive(Debug, Default, Clone)]
pub struct OrderOptions {
	pub disable_optimal_order_heuristic: bool,
}

#[derive(Debug, Default, Clone)]
#[expect(dead_code)]
struct LayerNode {
	// rank/min_rank/max_rank carried for parity with JS layer-graph node label;
	// ordering only consults `order` and borders.
	rank: Option<i32>,
	min_rank: Option<i32>,
	max_rank: Option<i32>,
	order: Option<usize>,
	border_left: Option<SmolStr>,
	border_right: Option<SmolStr>,
}

#[derive(Debug, Default, Clone)]
struct LayerEdge {
	weight: f64,
}

#[derive(Debug, Default, Clone)]
struct LayerGraphLabel {
	movable: Vec<NodeId>,
}

type LayerGraph = Graph<LayerGraphLabel, LayerNode, LayerEdge>;

#[derive(Debug, Clone, PartialEq)]
pub struct BarycenterEntry {
	pub v: NodeId,
	pub barycenter: Option<f64>,
	pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEntry {
	pub vs: Vec<NodeId>,
	pub i: usize,
	pub barycenter: Option<f64>,
	pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SortResult {
	pub vs: Vec<NodeId>,
	pub barycenter: Option<f64>,
	pub weight: Option<f64>,
}

// ---------- public test surface ------------------------------------------
//
// The order-pipeline's internal helpers operate on a small LayerGraph type
// that isn't exposed. For testing parity with the JS suite we offer thin
// adapters that take a standard layout graph instead.

/// Public barycenter: computes barycenter / weight per movable node. Reads
/// `order` from node labels and `weight` from edge labels.
pub fn barycenter(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	movable: &[NodeId],
) -> Vec<BarycenterEntry> {
	movable
		.iter()
		.map(|v| {
			let in_es = graph.in_edges(v).unwrap_or_default();
			if in_es.is_empty() {
				BarycenterEntry {
					v: v.clone(),
					barycenter: None,
					weight: None,
				}
			} else {
				let mut sum = 0.0;
				let mut weight = 0.0;
				for e in in_es {
					let edge_w = graph
						.edge_obj(&e)
						.map_or(0., |l| l.weight);
					let order = graph
						.node(&e.v)
						.and_then(|n| n.order)
						.unwrap_or(0) as f64;
					sum += edge_w * order;
					weight += edge_w;
				}
				BarycenterEntry {
					v: v.clone(),
					barycenter: Some(if weight > 0.0 {
						sum / weight
					} else {
						0.0
					}),
					weight: Some(weight),
				}
			}
		})
		.collect()
}

/// Public resolve-conflicts: takes a list of barycenter entries and a
/// constraint graph, returns coalesced entries respecting the constraints.
pub fn resolve_conflicts(
	entries: &[BarycenterEntry],
	constraint_graph: &Graph<(), (), ()>,
) -> Vec<ResolvedEntry> {
	resolve_conflicts_impl(entries, constraint_graph)
}

/// Public sort: takes resolved entries and a bias direction, returns a
/// sorted `SortResult`.
pub fn sort(entries: Vec<ResolvedEntry>, bias_right: bool) -> SortResult {
	sort_impl(entries, bias_right)
}

/// Public add-subgraph-constraints. Walks parents of each node to add
/// ordering edges into the constraint graph.
pub fn add_subgraph_constraints(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	constraint_graph: &mut Graph<(), (), ()>,
	vs: &[NodeId],
) {
	let mut prev: HashMap<NodeId, NodeId> = HashMap::new();
	let mut root_prev: Option<NodeId> = None;
	for v in vs {
		let mut child = graph.parent(v).map(NodeId::from);
		while let Some(c) = child.clone() {
			let parent = graph.parent(&c).map(NodeId::from);
			let prev_child = if let Some(p) = &parent {
				let pc = prev.get(p).cloned();
				prev.insert(p.clone(), c.clone());
				pc
			} else {
				let pc = root_prev;
				root_prev = Some(c.clone());
				pc
			};
			if let Some(pc) = prev_child
				&& pc != c
			{
				constraint_graph.set_edge(pc, c, ());
				break;
			}
			child = parent;
		}
	}
}

// ---------- main entry point ---------------------------------------------

pub fn order(
	graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	opts: &OrderOptions,
) {
	let max_rank = util::max_rank(graph);
	if max_rank < 0 {
		return;
	}
	let down_ranks = util::range(1, max_rank + 1, 1);
	let up_ranks = util::range(max_rank - 1, -1, -1);

	#[cfg(feature = "profile")]
	let t_blg = std::time::Instant::now();
	let by_rank = nodes_by_rank(graph);
	#[cfg(feature = "profile")]
	let dt_bucket = t_blg.elapsed();
	let mut down_layer_graphs =
		build_layer_graphs(graph, &down_ranks, Relationship::InEdges, &by_rank);
	let mut up_layer_graphs =
		build_layer_graphs(graph, &up_ranks, Relationship::OutEdges, &by_rank);
	#[cfg(feature = "profile")]
	let dt_blg = t_blg.elapsed();
	#[cfg(feature = "profile")]
	eprintln!(
		"[dagre]   order::pre nodes_by_rank={:.1}ms build_layer_graphs(x2)={:.1}ms",
		dt_bucket.as_secs_f64() * 1000.0,
		(dt_blg - dt_bucket).as_secs_f64() * 1000.0,
	);
	#[cfg(feature = "profile")]
	let t_init = std::time::Instant::now();
	let mut layering = init_order(graph);
	assign_order(graph, &layering);
	#[cfg(feature = "profile")]
	eprintln!(
		"[dagre]   order::pre init_order+assign={:.1}ms",
		t_init.elapsed().as_secs_f64() * 1000.0,
	);

	if opts.disable_optimal_order_heuristic {
		return;
	}

	let mut best_cc = f64::INFINITY;
	let mut best: Option<Vec<Vec<NodeId>>> = None;

	let mut last_best = 0;
	let mut i = 0;
	#[cfg(feature = "profile")]
	let (mut t_sweep, mut t_layer, mut t_cc, mut t_clone) =
		(0u128, 0u128, 0u128, 0u128);
	while last_best < 4 {
		let lgs = if i % 2 == 1 {
			&mut down_layer_graphs
		} else {
			&mut up_layer_graphs
		};
		#[cfg(feature = "profile")]
		let t0 = std::time::Instant::now();
		sweep_layer_graphs(lgs, i % 4 >= 2, graph);
		#[cfg(feature = "profile")]
		{
			t_sweep += t0.elapsed().as_nanos();
		}
		#[cfg(feature = "profile")]
		let t1 = std::time::Instant::now();
		layering = util::build_layer_matrix(graph);
		#[cfg(feature = "profile")]
		{
			t_layer += t1.elapsed().as_nanos();
		}
		#[cfg(feature = "profile")]
		let t2 = std::time::Instant::now();
		let cc = cross_count(graph, &layering) as f64;
		#[cfg(feature = "profile")]
		{
			t_cc += t2.elapsed().as_nanos();
		}
		#[cfg(feature = "profile")]
		let t3 = std::time::Instant::now();
		if cc < best_cc {
			last_best = 0;
			best = Some(layering.clone());
			best_cc = cc;
		} else if (cc - best_cc).abs() < f64::EPSILON {
			best = Some(layering.clone());
			last_best += 1;
		} else {
			last_best += 1;
		}
		#[cfg(feature = "profile")]
		{
			t_clone += t3.elapsed().as_nanos();
		}
		i += 1;
	}
	#[cfg(feature = "profile")]
	eprintln!(
		"[dagre]   order iters={} sweep={:.1}ms layer={:.1}ms cc={:.1}ms clone={:.1}ms",
		i,
		t_sweep as f64 / 1e6,
		t_layer as f64 / 1e6,
		t_cc as f64 / 1e6,
		t_clone as f64 / 1e6,
	);

	if let Some(b) = best {
		assign_order(graph, &b);
	}
}

#[derive(Clone, Copy)]
enum Relationship {
	InEdges,
	OutEdges,
}

fn nodes_by_rank(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> HashMap<i32, Vec<NodeId>> {
	let mut nodes_by_rank: HashMap<i32, Vec<NodeId>> = HashMap::new();
	for v in graph.nodes_iter() {
		if let Some(n) = graph.node(v) {
			if let Some(r) = n.rank {
				nodes_by_rank
					.entry(r)
					.or_default()
					.push(v.into());
			}
			if let (Some(min), Some(max)) = (n.min_rank, n.max_rank) {
				for r in min..=max {
					if Some(r) != n.rank {
						nodes_by_rank
							.entry(r)
							.or_default()
							.push(v.into());
					}
				}
			}
		}
	}
	nodes_by_rank
}

fn build_layer_graphs(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	ranks: &[i32],
	relationship: Relationship,
	nodes_by_rank: &HashMap<i32, Vec<NodeId>>,
) -> Vec<LayerGraph> {
	let empty: Vec<NodeId> = Vec::new();
	ranks
		.iter()
		.map(|r| {
			build_layer_graph(
				graph,
				*r,
				relationship,
				nodes_by_rank.get(r).unwrap_or(&empty),
			)
		})
		.collect()
}

fn build_layer_graph(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	rank: i32,
	relationship: Relationship,
	nodes_with_rank: &[NodeId],
) -> LayerGraph {
	let mut result: LayerGraph = Graph::with_opts(GraphOpts {
		directed: true,
		multigraph: false,
		compound: false,
	});
	let mut movable: Vec<NodeId> = Vec::with_capacity(nodes_with_rank.len());

	for v in nodes_with_rank {
		// Read only the fields we need by reference. Cloning the whole
		// NodeLabel here was ~1s across the two pre-loop build_layer_graphs
		// calls because NodeLabel contains Option<Vec<String>> and
		// Option<Box<EdgeLabel>>.
		let (n_rank, n_min_rank, n_max_rank, n_order, border) = match graph
			.node(v)
		{
			Some(n) => {
				let ri = rank as usize;
				let bord = match (&n.border_left, &n.border_right) {
					(Some(bl), Some(br)) if ri < bl.len() && ri < br.len() => {
						Some((bl[ri].clone(), br[ri].clone()))
					}
					_ => None,
				};
				(n.rank, n.min_rank, n.max_rank, n.order, bord)
			}
			None => continue,
		};
		let in_range = n_rank == Some(rank)
			|| (n_min_rank.is_some()
				&& n_max_rank.is_some()
				&& n_min_rank.unwrap() <= rank
				&& rank <= n_max_rank.unwrap());
		if !in_range {
			continue;
		}
		result.set_node(
			v.clone(),
			LayerNode {
				rank: n_rank,
				min_rank: n_min_rank,
				max_rank: n_max_rank,
				order: n_order,
				..Default::default()
			},
		);
		movable.push(v.clone());

		let es = match relationship {
			Relationship::InEdges => graph.in_edges(v).unwrap_or_default(),
			Relationship::OutEdges => graph.out_edges(v).unwrap_or_default(),
		};
		for e in es {
			let u = if e.v == *v { e.w.clone() } else { e.v.clone() };
			if !result.has_node(&u) {
				result.set_node(u.clone(), LayerNode::default());
			}
			let prev = result
				.edge(&u, v)
				.map_or(0., |l| l.weight);
			let w = graph
				.edge_obj(&e)
				.map_or(0., |l| l.weight);
			result.set_edge(
				u.clone(),
				v.clone(),
				LayerEdge { weight: prev + w },
			);
		}

		if let Some((bl, br)) = border
			&& let Some(n) = result.node_mut(v)
		{
			n.border_left = Some(bl);
			n.border_right = Some(br);
		}
	}
	result.set_graph(LayerGraphLabel { movable });
	result
}

// ---------- init order ---------------------------------------------------

/// Public init-order — visible for testing. Performs a DFS starting at the
/// leftmost rank node, assigning visit order to each non-subgraph node.
pub fn init_order(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Vec<Vec<NodeId>> {
	let mut visited: HashSet<NodeId> = HashSet::new();
	let simple_nodes: Vec<NodeId> = graph
		.nodes()
		.into_iter()
		.filter(|v| graph.children(Some(v)).is_empty())
		.collect();
	let max_rank = simple_nodes
		.iter()
		.filter_map(|v| graph.node(v).and_then(|n| n.rank))
		.max()
		.unwrap_or(0);
	let mut layers: Vec<Vec<NodeId>> = (0..=max_rank.max(0))
		.map(|_| Vec::new())
		.collect();
	if max_rank < 0 {
		return Vec::new();
	}

	fn dfs(
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		v: &str,
		visited: &mut HashSet<NodeId>,
		layers: &mut Vec<Vec<NodeId>>,
	) {
		if visited.contains(v) {
			return;
		}
		visited.insert(v.into());
		if let Some(n) = graph.node(v)
			&& let Some(r) = n.rank
			&& r >= 0
		{
			let ri = r as usize;
			while layers.len() <= ri {
				layers.push(Vec::new());
			}
			layers[ri].push(v.into());
		}
		let succ = graph.successors(v).unwrap_or_default();
		for s in succ {
			dfs(graph, &s, visited, layers);
		}
	}

	let mut ordered = simple_nodes;
	ordered.sort_by_key(|v| {
		graph
			.node(v)
			.and_then(|n| n.rank)
			.unwrap_or(0)
	});
	for v in ordered {
		dfs(graph, &v, &mut visited, &mut layers);
	}
	layers
}

fn assign_order(
	graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeId>],
) {
	for layer in layering {
		for (i, v) in layer.iter().enumerate() {
			if let Some(n) = graph.node_mut(v) {
				n.order = Some(i);
			}
		}
	}
}

// ---------- crossing count -----------------------------------------------

pub fn cross_count(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeId>],
) -> u64 {
	let mut cc = 0u64;
	for i in 1..layering.len() {
		cc += two_layer_cross_count(graph, &layering[i - 1], &layering[i]);
	}
	cc
}

fn two_layer_cross_count(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	north: &[NodeId],
	south: &[NodeId],
) -> u64 {
	if south.is_empty() {
		return 0;
	}
	let south_pos: HashMap<&str, usize> = south
		.iter()
		.enumerate()
		.map(|(i, v)| (v.as_str(), i))
		.collect();
	let mut south_entries: Vec<(usize, f64)> = Vec::new();
	let mut local: Vec<(usize, f64)> = Vec::new();
	for v in north {
		local.clear();
		if let Some(it) = graph.out_edges_iter(v) {
			for e in it {
				if let Some(&pos) = south_pos.get(e.w.as_str()) {
					let weight = graph
						.edge_obj(e)
						.map_or(0., |l| l.weight);
					local.push((pos, weight));
				}
			}
		}
		local.sort_by_key(|x| x.0);
		south_entries.extend(local.iter().copied());
	}

	let mut first_index = 1usize;
	while first_index < south.len() {
		first_index <<= 1;
	}
	let tree_size = 2 * first_index - 1;
	first_index -= 1;
	let mut tree = vec![0.0_f64; tree_size];
	let mut cc = 0.0f64;
	for (pos, weight) in south_entries {
		let mut index = pos + first_index;
		tree[index] += weight;
		let mut weight_sum = 0.0;
		while index > 0 {
			if index % 2 == 1 {
				weight_sum += tree[index + 1];
			}
			index = (index - 1) >> 1;
			tree[index] += weight;
		}
		cc = weight.mul_add(weight_sum, cc);
	}
	cc as u64
}

// ---------- barycenter (internal LayerGraph version) ---------------------

fn barycenter_impl(
	graph: &LayerGraph,
	movable: &[NodeId],
) -> Vec<BarycenterEntry> {
	movable
		.iter()
		.map(|v| {
			let mut sum = 0.0;
			let mut weight = 0.0;
			let mut any = false;
			if let Some(it) = graph.in_edges_iter(v) {
				for e in it {
					any = true;
					let edge_w = graph
						.edge_obj(e)
						.map_or(0., |l| l.weight);
					let order = graph
						.node(&e.v)
						.and_then(|n| n.order)
						.unwrap_or(0) as f64;
					sum += edge_w * order;
					weight += edge_w;
				}
			}
			if any {
				BarycenterEntry {
					v: v.clone(),
					barycenter: Some(if weight > 0.0 {
						sum / weight
					} else {
						0.0
					}),
					weight: Some(weight),
				}
			} else {
				BarycenterEntry {
					v: v.clone(),
					barycenter: None,
					weight: None,
				}
			}
		})
		.collect()
}

// ---------- resolve conflicts --------------------------------------------

// TODO: refactor
#[expect(clippy::too_many_lines)]
fn resolve_conflicts_impl(
	entries: &[BarycenterEntry],
	constraint_graph: &Graph<(), (), ()>,
) -> Vec<ResolvedEntry> {
	#[derive(Debug)]
	struct Mapped {
		indegree: usize,
		ins: Vec<usize>,
		outs: Vec<usize>,
		vs: Vec<NodeId>,
		i: usize,
		barycenter: Option<f64>,
		weight: Option<f64>,
		merged: bool,
	}

	let mut mapped: Vec<Mapped> = Vec::with_capacity(entries.len());
	let mut v_to_idx: HashMap<NodeId, usize> = HashMap::new();
	for (i, e) in entries.iter().enumerate() {
		v_to_idx.insert(e.v.clone(), i);
		mapped.push(Mapped {
			indegree: 0,
			ins: Vec::new(),
			outs: Vec::new(),
			vs: vec![e.v.clone()],
			i,
			barycenter: e.barycenter,
			weight: e.weight,
			merged: false,
		});
	}

	for e in constraint_graph.edges() {
		let (Some(&vi), Some(&wi)) = (v_to_idx.get(&e.v), v_to_idx.get(&e.w))
		else {
			continue;
		};
		mapped[wi].indegree += 1;
		mapped[vi].outs.push(wi);
	}

	let mut source_set: Vec<usize> = mapped
		.iter()
		.enumerate()
		.filter(|(_, m)| m.indegree == 0)
		.map(|(i, _)| i)
		.collect();

	fn merge_entries(m: &mut [Mapped], target: usize, source: usize) {
		let mut sum = 0.0;
		let mut weight = 0.0;
		if let (Some(b), Some(w)) = (m[target].barycenter, m[target].weight)
			&& w != 0.0
		{
			sum = b.mul_add(w, sum);
			weight += w;
		}
		if let (Some(b), Some(w)) = (m[source].barycenter, m[source].weight)
			&& w != 0.0
		{
			sum = b.mul_add(w, sum);
			weight += w;
		}
		let source_vs = mem::take(&mut m[source].vs);
		let new_vs = {
			let mut v = source_vs;
			v.extend(mem::take(&mut m[target].vs));
			v
		};
		m[target].vs = new_vs;
		m[target].barycenter = if weight > 0.0 {
			Some(sum / weight)
		} else {
			None
		};
		m[target].weight = if weight > 0.0 { Some(weight) } else { None };
		m[target].i = m[target].i.min(m[source].i);
		m[source].merged = true;
	}

	let mut entries_out: Vec<usize> = Vec::new();
	while let Some(idx) = source_set.pop() {
		entries_out.push(idx);
		// Handle ins (reverse).
		let ins: Vec<usize> = mapped[idx]
			.ins
			.iter()
			.copied()
			.rev()
			.collect();
		for u in ins {
			if mapped[u].merged {
				continue;
			}
			let merge = match (mapped[u].barycenter, mapped[idx].barycenter) {
				(Some(ub), Some(vb)) => ub >= vb,
				_ => true,
			};
			if merge {
				merge_entries(&mut mapped, idx, u);
			}
		}
		// Handle outs.
		let outs: Vec<usize> = mapped[idx].outs.clone();
		for w in outs {
			mapped[w].ins.push(idx);
			mapped[w].indegree = mapped[w].indegree.saturating_sub(1);
			if mapped[w].indegree == 0 {
				source_set.push(w);
			}
		}
	}

	entries_out
		.into_iter()
		.filter(|&i| !mapped[i].merged)
		.map(|i| ResolvedEntry {
			vs: mapped[i].vs.clone(),
			i: mapped[i].i,
			barycenter: mapped[i].barycenter,
			weight: mapped[i].weight,
		})
		.collect()
}

// ---------- sort --------------------------------------------------------

fn sort_impl(entries: Vec<ResolvedEntry>, bias_right: bool) -> SortResult {
	let (sortable, mut unsortable) =
		util::partition(entries, |e| e.barycenter.is_some());
	unsortable.sort_by_key(|e| cmp::Reverse(e.i));
	let mut sortable = sortable;
	sortable.sort_by(|a, b| {
		let ab = a.barycenter.unwrap();
		let bb = b.barycenter.unwrap();
		if ab < bb {
			cmp::Ordering::Less
		} else if ab > bb {
			cmp::Ordering::Greater
		} else if !bias_right {
			a.i.cmp(&b.i)
		} else {
			b.i.cmp(&a.i)
		}
	});

	let mut vs: Vec<Vec<NodeId>> = Vec::new();
	let mut sum = 0.0;
	let mut weight = 0.0;
	let mut vs_index = 0usize;

	let consume_unsortable = |vs: &mut Vec<Vec<NodeId>>,
	                          unsortable: &mut Vec<ResolvedEntry>,
	                          mut index: usize|
	 -> usize {
		while let Some(last) = unsortable.last() {
			if last.i <= index {
				let last = unsortable.pop().unwrap();
				let n = last.vs.len();
				vs.push(last.vs);
				index += n;
			} else {
				break;
			}
		}
		index
	};

	vs_index = consume_unsortable(&mut vs, &mut unsortable, vs_index);

	for entry in sortable {
		vs_index += entry.vs.len();
		let bc = entry.barycenter.unwrap();
		let w = entry.weight.unwrap_or(0.0);
		sum = bc.mul_add(w, sum);
		weight += w;
		vs.push(entry.vs);
		vs_index = consume_unsortable(&mut vs, &mut unsortable, vs_index);
	}

	let flat: Vec<NodeId> = vs.into_iter().flatten().collect();
	SortResult {
		vs: flat,
		barycenter: if weight > 0.0 {
			Some(sum / weight)
		} else {
			None
		},
		weight: if weight > 0.0 { Some(weight) } else { None },
	}
}

// ---------- sort subgraph ------------------------------------------------

fn sort_subgraph(
	graph: &LayerGraph,
	movable: &[NodeId],
	constraint_graph: &Graph<(), (), ()>,
	bias_right: bool,
) -> SortResult {
	let barycenters = barycenter_impl(graph, movable);
	let entries = barycenters;
	let subgraphs: HashMap<NodeId, SortResult> = HashMap::new();
	// Layer graphs from build_layer_graph are flat (compound-but-rooted), no
	// nested subgraphs beyond the root level in our scope. So skip subgraph
	// recursion here. (Compound dagre input is out of scope per user choice.)
	let _ = subgraphs;

	let resolved = resolve_conflicts_impl(&entries, constraint_graph);
	sort_impl(resolved, bias_right)
}

// ---------- add subgraph constraints (no-op in our scope) -----------------

const fn add_subgraph_constraints_layer(
	_layer: &LayerGraph,
	_cg: &mut Graph<(), (), ()>,
	_vs: &[NodeId],
) {
	// No compound subgraphs to add constraints for in our scope.
}

// ---------- sweep --------------------------------------------------------

fn sweep_layer_graphs(
	layer_graphs: &mut [LayerGraph],
	bias_right: bool,
	graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
) {
	let mut cg: Graph<(), (), ()> = Graph::new();
	for lg in layer_graphs {
		let movable = lg
			.graph()
			.map(|g| g.movable.clone())
			.unwrap_or_default();
		let sorted = sort_subgraph(lg, &movable, &cg, bias_right);
		for (i, v) in sorted.vs.iter().enumerate() {
			if let Some(n) = lg.node_mut(v) {
				n.order = Some(i);
			}
			if let Some(n) = graph.node_mut(v) {
				n.order = Some(i);
			}
		}
		add_subgraph_constraints_layer(lg, &mut cg, &sorted.vs);
	}
}
