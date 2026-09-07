//! Crossing-minimization order pipeline - port of `lib/order/*.ts`.

use rustc_hash::FxHashMap;

use crate::{
	graph::{Graph, GraphOpts, NodeIdx},
	types::{EdgeLabel, GraphLabel, NodeLabel},
	util,
};
use std::{cmp, mem};

#[derive(Debug, Default, Clone)]
pub struct OrderOptions {
	pub disable_optimal_order_heuristic: bool,
}

#[derive(Debug, Default, Clone)]
#[expect(dead_code)]
struct LayerNode {
	// rank/min_rank/max_rank carried for parity with JS layer-graph node label;
	// ordering only consults the borders.
	rank: Option<i32>,
	min_rank: Option<i32>,
	max_rank: Option<i32>,
	border_left: Option<NodeIdx>,
	border_right: Option<NodeIdx>,
}

#[derive(Debug, Default, Clone)]
struct LayerEdge {
	weight: f64,
}

#[derive(Debug, Default, Clone)]
struct LayerGraphLabel {
	/// Movable nodes, as *local* layer-graph indices.
	movable: Vec<NodeIdx>,
	/// Local index -> index in the parent layout graph.
	global: Vec<NodeIdx>,
}

/// A layer graph uses its own compact index space rather than sharing the
/// parent's: there is one per rank (~200 for a large bundle) and the parent
/// has ~63k node slots after normalize, so sharing would allocate 200 x 63k
/// slots. `LayerGraphLabel::global` maps back.
type LayerGraph = Graph<LayerGraphLabel, LayerNode, LayerEdge>;

#[derive(Debug, Clone, PartialEq)]
pub struct BarycenterEntry {
	pub v: NodeIdx,
	pub barycenter: Option<f64>,
	pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEntry {
	pub vs: Vec<NodeIdx>,
	pub i: usize,
	pub barycenter: Option<f64>,
	pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SortResult {
	pub vs: Vec<NodeIdx>,
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
	movable: &[NodeIdx],
) -> Vec<BarycenterEntry> {
	movable
		.iter()
		.map(|&v| {
			let in_es = graph.in_edges(v).unwrap_or_default();
			if in_es.is_empty() {
				BarycenterEntry {
					v,
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
						.node(e.v)
						.and_then(|n| n.order)
						.unwrap_or(0) as f64;
					sum = edge_w.mul_add(order, sum);
					weight += edge_w;
				}
				BarycenterEntry {
					v,
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
	vs: &[NodeIdx],
) {
	let mut prev: FxHashMap<NodeIdx, NodeIdx> = FxHashMap::default();
	let mut root_prev: Option<NodeIdx> = None;
	for &v in vs {
		let mut child = graph.parent(v);
		while let Some(c) = child {
			let parent = graph.parent(c);
			let prev_child = if let Some(p) = parent {
				let pc = prev.get(&p).copied();
				prev.insert(p, c);
				pc
			} else {
				let pc = root_prev;
				root_prev = Some(c);
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
	let mut best: Option<Vec<Vec<NodeIdx>>> = None;

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
		};
		#[cfg(feature = "profile")]
		let t1 = std::time::Instant::now();
		layering = util::build_layer_matrix(graph);
		#[cfg(feature = "profile")]
		{
			t_layer += t1.elapsed().as_nanos();
		};
		#[cfg(feature = "profile")]
		let t2 = std::time::Instant::now();
		let cc = cross_count(graph, &layering) as f64;
		#[cfg(feature = "profile")]
		{
			t_cc += t2.elapsed().as_nanos();
		};
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
		};
		i += 1;
	}
	#[cfg(feature = "profile")]
	eprintln!(
		"[dagre]   order iters={} best_crossings={} sweep={:.1}ms layer={:.1}ms cc={:.1}ms clone={:.1}ms",
		i,
		best_cc,
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
) -> FxHashMap<i32, Vec<NodeIdx>> {
	let mut nodes_by_rank: FxHashMap<i32, Vec<NodeIdx>> = FxHashMap::default();
	for v in graph.nodes_iter() {
		if let Some(n) = graph.node(v) {
			if let Some(r) = n.rank {
				nodes_by_rank
					.entry(r)
					.or_default()
					.push(v);
			}
			if let (Some(min), Some(max)) = (n.min_rank, n.max_rank) {
				for r in min..=max {
					if Some(r) != n.rank {
						nodes_by_rank
							.entry(r)
							.or_default()
							.push(v);
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
	nodes_by_rank: &FxHashMap<i32, Vec<NodeIdx>>,
) -> Vec<LayerGraph> {
	let empty: Vec<NodeIdx> = Vec::new();
	// Scratch global -> local map, reused across every rank. `u32::MAX` means
	// "not in this layer graph"; entries are cleared per rank by walking the
	// nodes we actually touched.
	let mut local_of: Vec<u32> = vec![u32::MAX; graph.node_bound()];
	ranks
		.iter()
		.map(|r| {
			build_layer_graph(
				graph,
				*r,
				relationship,
				nodes_by_rank.get(r).unwrap_or(&empty),
				&mut local_of,
			)
		})
		.collect()
}

#[expect(clippy::too_many_lines)]
fn build_layer_graph(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	rank: i32,
	relationship: Relationship,
	nodes_with_rank: &[NodeIdx],
	local_of: &mut [u32],
) -> LayerGraph {
	let mut result: LayerGraph = Graph::with_opts(GraphOpts {
		directed: true,
		multigraph: false,
		compound: false,
	});
	let mut movable: Vec<NodeIdx> = Vec::with_capacity(nodes_with_rank.len());
	let mut global: Vec<NodeIdx> = Vec::with_capacity(nodes_with_rank.len());

	// Interns a global node into this layer graph's local index space.
	fn intern(
		local_of: &mut [u32],
		global: &mut Vec<NodeIdx>,
		result: &mut LayerGraph,
		g: NodeIdx,
		label: LayerNode,
	) -> NodeIdx {
		let slot = local_of[g.index()];
		if slot == u32::MAX {
			let l = result.add_node(label);
			debug_assert_eq!(l.index(), global.len());
			global.push(g);
			local_of[g.index()] = l.0;
			l
		} else {
			let l = NodeIdx(slot);
			result.set_node(l, label);
			l
		}
	}

	for &v in nodes_with_rank {
		// Read only the fields we need by reference. Cloning the whole
		// NodeLabel here was ~1s across the two pre-loop build_layer_graphs
		// calls because NodeLabel contains Option<Vec<_>> and
		// Option<Box<EdgeLabel>>.
		let (n_rank, n_min_rank, n_max_rank, border) = match graph.node(v) {
			Some(n) => {
				let ri = rank as usize;
				let bord = match (&n.border_left, &n.border_right) {
					(Some(bl), Some(br)) if ri < bl.len() && ri < br.len() => {
						Some((bl[ri], br[ri]))
					}
					_ => None,
				};
				(n.rank, n.min_rank, n.max_rank, bord)
			}
			None => continue,
		};
		let in_range = n_rank == Some(rank)
			|| (n_min_rank.is_some()
				&& n_max_rank.is_some()
				&& n_min_rank.unwrap_or(i32::MAX) <= rank
				&& rank <= n_max_rank.unwrap_or(i32::MIN));
		if !in_range {
			continue;
		}
		let lv = intern(
			local_of,
			&mut global,
			&mut result,
			v,
			LayerNode {
				rank: n_rank,
				min_rank: n_min_rank,
				max_rank: n_max_rank,
				..Default::default()
			},
		);
		movable.push(lv);

		let es = match relationship {
			Relationship::InEdges => graph.in_edge_idxs(v),
			Relationship::OutEdges => graph.out_edge_idxs(v),
		};
		for &ei in es {
			let Some((e, label)) = graph.edge_entry(ei) else {
				continue;
			};
			let u = if e.v == v { e.w } else { e.v };
			let lu = if local_of[u.index()] == u32::MAX {
				intern(
					local_of,
					&mut global,
					&mut result,
					u,
					LayerNode::default(),
				)
			} else {
				NodeIdx(local_of[u.index()])
			};
			let prev = result
				.edge(lu, lv)
				.map_or(0., |l| l.weight);
			result.set_edge(
				lu,
				lv,
				LayerEdge {
					weight: prev + label.weight,
				},
			);
		}

		if let Some((bl, br)) = border
			&& let Some(n) = result.node_mut(lv)
		{
			n.border_left = Some(bl);
			n.border_right = Some(br);
		}
	}

	// Reset the scratch map for the next rank.
	for &g in &global {
		local_of[g.index()] = u32::MAX;
	}

	result.set_graph(LayerGraphLabel { movable, global });
	result
}

// ---------- init order ---------------------------------------------------

/// Public init-order - visible for testing. Performs a DFS starting at the
/// leftmost rank node, assigning visit order to each non-subgraph node.
pub fn init_order(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> Vec<Vec<NodeIdx>> {
	let mut visited = vec![false; graph.node_bound()];
	let simple_nodes: Vec<NodeIdx> = graph
		.nodes_iter()
		.filter(|&v| graph.children(Some(v)).is_empty())
		.collect();
	let max_rank = simple_nodes
		.iter()
		.filter_map(|&v| graph.node(v).and_then(|n| n.rank))
		.max()
		.unwrap_or(0);
	let mut layers: Vec<Vec<NodeIdx>> = (0..=max_rank.max(0))
		.map(|_| Vec::new())
		.collect();
	if max_rank < 0 {
		return Vec::new();
	}

	fn dfs(
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		start: NodeIdx,
		visited: &mut [bool],
		layers: &mut Vec<Vec<NodeIdx>>,
	) {
		// Explicit stack: normalize turns long edges into dummy chains as long
		// as the rank span, so a recursive DFS blows the stack on large graphs.
		let mut stack = vec![start];
		while let Some(v) = stack.pop() {
			if visited[v.index()] {
				continue;
			}
			visited[v.index()] = true;
			if let Some(n) = graph.node(v)
				&& let Some(r) = n.rank
				&& r >= 0
			{
				let ri = r as usize;
				while layers.len() <= ri {
					layers.push(Vec::new());
				}
				layers[ri].push(v);
			}
			// Reversed so the first successor is visited first, matching the
			// recursive traversal this replaced.
			let succ = graph.successors(v).unwrap_or_default();
			stack.extend(succ.into_iter().rev());
		}
	}

	let mut ordered = simple_nodes;
	ordered.sort_by_key(|&v| {
		graph
			.node(v)
			.and_then(|n| n.rank)
			.unwrap_or(0)
	});
	for v in ordered {
		dfs(graph, v, &mut visited, &mut layers);
	}
	layers
}

fn assign_order(
	graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeIdx>],
) {
	for layer in layering {
		for (i, &v) in layer.iter().enumerate() {
			if let Some(n) = graph.node_mut(v) {
				n.order = Some(i);
			}
		}
	}
}

// ---------- crossing count -----------------------------------------------

pub fn cross_count(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeIdx>],
) -> u64 {
	let mut cc = 0u64;
	// Scratch buffers, reused across every layer pair.
	let mut scratch = CrossCountScratch {
		south_pos: vec![u32::MAX; graph.node_bound()],
		south_entries: Vec::new(),
		local: Vec::new(),
		tree: Vec::new(),
	};
	for i in 1..layering.len() {
		cc += two_layer_cross_count(
			graph,
			&layering[i - 1],
			&layering[i],
			&mut scratch,
		);
	}
	cc
}

struct CrossCountScratch {
	/// Node index -> position in the south layer; `u32::MAX` is "not there".
	south_pos: Vec<u32>,
	south_entries: Vec<(usize, f64)>,
	local: Vec<(usize, f64)>,
	/// Fenwick tree of accumulated weights.
	tree: Vec<f64>,
}

fn two_layer_cross_count(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	north: &[NodeIdx],
	south: &[NodeIdx],
	scratch: &mut CrossCountScratch,
) -> u64 {
	if south.is_empty() {
		return 0;
	}
	let CrossCountScratch {
		south_pos,
		south_entries,
		local,
		tree,
	} = scratch;
	for (i, &v) in south.iter().enumerate() {
		south_pos[v.index()] = u32::try_from(i).unwrap_or(u32::MAX);
	}
	south_entries.clear();
	for &v in north {
		local.clear();
		for &ei in graph.out_edge_idxs(v) {
			let Some((e, label)) = graph.edge_entry(ei) else {
				continue;
			};
			let pos = south_pos[e.w.index()];
			if pos != u32::MAX {
				local.push((pos as usize, label.weight));
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
	tree.clear();
	tree.resize(tree_size, 0.0);
	let mut cc = 0.0f64;
	for &(pos, weight) in south_entries.iter() {
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

	// Clear the scratch positions we set.
	for &v in south {
		south_pos[v.index()] = u32::MAX;
	}
	cc as u64
}

// ---------- barycenter (internal LayerGraph version) ---------------------

/// `order` is read from the *main* graph through `global`, not from the layer
/// graph's own node labels. The layer graphs are built once and reused across
/// every sweep, so their copies of `order` go stale immediately - and fixed
/// (non-movable) neighbours never had one at all, which pinned every
/// barycenter to 0 and made the sweep close to a no-op. JS dagre gets this for
/// free by aliasing the main graph's label objects into the layer graph.
fn barycenter_impl(
	layer: &LayerGraph,
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	global: &[NodeIdx],
	movable: &[NodeIdx],
) -> Vec<BarycenterEntry> {
	movable
		.iter()
		.map(|&v| {
			let mut sum = 0.0;
			let mut weight = 0.0;
			let mut any = false;
			for &ei in layer.in_edge_idxs(v) {
				let Some((e, label)) = layer.edge_entry(ei) else {
					continue;
				};
				any = true;
				let order = global
					.get(e.v.index())
					.and_then(|&g| graph.node(g))
					.and_then(|n| n.order)
					.unwrap_or(0) as f64;
				sum = label.weight.mul_add(order, sum);
				weight += label.weight;
			}
			if any {
				BarycenterEntry {
					v,
					barycenter: Some(if weight > 0.0 {
						sum / weight
					} else {
						0.0
					}),
					weight: Some(weight),
				}
			} else {
				BarycenterEntry {
					v,
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
		vs: Vec<NodeIdx>,
		i: usize,
		barycenter: Option<f64>,
		weight: Option<f64>,
		merged: bool,
	}

	let mut mapped: Vec<Mapped> = Vec::with_capacity(entries.len());
	// Entry ids are dense indices into whichever graph produced them, so a
	// flat lookup beats a hash map. `u32::MAX` means "not an entry".
	let bound = entries
		.iter()
		.map(|e| e.v.index() + 1)
		.max()
		.unwrap_or(0);
	let mut v_to_idx: Vec<u32> = vec![u32::MAX; bound];
	for (i, e) in entries.iter().enumerate() {
		v_to_idx[e.v.index()] = u32::try_from(i).unwrap_or(u32::MAX);
		mapped.push(Mapped {
			indegree: 0,
			ins: Vec::new(),
			outs: Vec::new(),
			vs: vec![e.v],
			i,
			barycenter: e.barycenter,
			weight: e.weight,
			merged: false,
		});
	}

	for e in constraint_graph.edges_iter() {
		let (Some(&vi), Some(&wi)) =
			(v_to_idx.get(e.v.index()), v_to_idx.get(e.w.index()))
		else {
			continue;
		};
		if vi == u32::MAX || wi == u32::MAX {
			continue;
		}
		let (vi, wi) = (vi as usize, wi as usize);
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
		let ab = a.barycenter.unwrap_or(0.0);
		let bb = b.barycenter.unwrap_or(0.0);
		if ab < bb {
			cmp::Ordering::Less
		} else if ab > bb {
			cmp::Ordering::Greater
		} else if bias_right {
			b.i.cmp(&a.i)
		} else {
			a.i.cmp(&b.i)
		}
	});

	let mut vs: Vec<Vec<NodeIdx>> = Vec::new();
	let mut sum = 0.0;
	let mut weight = 0.0;
	let mut vs_index = 0usize;

	let consume_unsortable = |vs: &mut Vec<Vec<NodeIdx>>,
	                          unsortable: &mut Vec<ResolvedEntry>,
	                          mut index: usize|
	 -> usize {
		while let Some(last) = unsortable.last() {
			if last.i <= index {
				if let Some(last) = unsortable.pop() {
					let n = last.vs.len();
					vs.push(last.vs);
					index += n;
				}
			} else {
				break;
			}
		}
		index
	};

	vs_index = consume_unsortable(&mut vs, &mut unsortable, vs_index);

	for entry in sortable {
		vs_index += entry.vs.len();
		let bc = entry.barycenter.unwrap_or(0.0);
		let w = entry.weight.unwrap_or(0.0);
		sum = bc.mul_add(w, sum);
		weight += w;
		vs.push(entry.vs);
		vs_index = consume_unsortable(&mut vs, &mut unsortable, vs_index);
	}

	let flat: Vec<NodeIdx> = vs.into_iter().flatten().collect();
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
	layer: &LayerGraph,
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	global: &[NodeIdx],
	movable: &[NodeIdx],
	constraint_graph: &Graph<(), (), ()>,
	bias_right: bool,
) -> SortResult {
	let entries = barycenter_impl(layer, graph, global, movable);
	// Layer graphs from build_layer_graph are flat (compound-but-rooted), no
	// nested subgraphs beyond the root level in our scope. So skip subgraph
	// recursion here. (Compound dagre input is out of scope per user choice.)
	let resolved = resolve_conflicts_impl(&entries, constraint_graph);
	sort_impl(resolved, bias_right)
}

// ---------- add subgraph constraints (no-op in our scope) -----------------

const fn add_subgraph_constraints_layer(
	_layer: &LayerGraph,
	_cg: &mut Graph<(), (), ()>,
	_vs: &[NodeIdx],
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
		let (movable, global) = lg
			.graph()
			.map(|g| (g.movable.clone(), g.global.clone()))
			.unwrap_or_default();
		let sorted =
			sort_subgraph(lg, graph, &global, &movable, &cg, bias_right);
		// Only the main graph carries `order` now; the next layer's
		// barycenters read it back from there.
		for (i, &v) in sorted.vs.iter().enumerate() {
			if let Some(&g) = global.get(v.index())
				&& let Some(n) = graph.node_mut(g)
			{
				n.order = Some(i);
			}
		}
		add_subgraph_constraints_layer(lg, &mut cg, &sorted.vs);
	}
}
