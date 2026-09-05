//! Position assignment - port of `lib/position/*.ts`.
//!
//! `positionY` assigns y by stacking ranks with `ranksep` between rows.
//! `positionX` uses the Brandes & Kopf algorithm to compute four extreme
//! alignments (up/down x left/right) and balances among them.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
	graph::{Graph, GraphOpts, NodeIdx},
	types::{
		Align,
		BorderType,
		Dummy,
		EdgeLabel,
		GraphLabel,
		LabelPos,
		NodeLabel,
		RankAlign,
	},
	util,
};
use std::{cmp::Ordering, collections, hash::BuildHasher};

pub fn position(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	// Work on a non-compound copy as the JS does.
	let mut g = util::as_non_compound_graph(graph);
	position_y(&mut g);
	let xs = position_x(&g);
	// Copy y from non-compound back, x from xs. `as_non_compound_graph`
	// preserves node indices, so this is index-for-index.
	for v in g.nodes() {
		if let Some(yn) = g.node(v).and_then(|n| n.y)
			&& let Some(target) = graph.node_mut(v)
		{
			target.y = Some(yn);
		}
		if let Some(&x) = xs.get(v)
			&& let Some(target) = graph.node_mut(v)
		{
			target.x = Some(x);
		}
	}
}

fn position_y(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let layering = util::build_layer_matrix(graph);
	let (rank_sep, rank_align) = {
		let gl = graph
			.graph()
			.cloned()
			.unwrap_or_default();
		(
			gl.ranksep.unwrap_or(50.0),
			gl.rank_align
				.unwrap_or(RankAlign::Center),
		)
	};
	let mut prev_y = 0.0_f64;
	for layer in layering {
		let max_h = layer
			.iter()
			.map(|&v| graph.node(v).map_or(0., |n| n.height))
			.fold(0.0f64, f64::max);
		for &v in &layer {
			let h = graph.node(v).map_or(0., |n| n.height);
			if let Some(n) = graph.node_mut(v) {
				n.y = Some(match rank_align {
					RankAlign::Top => prev_y + h / 2.0,
					RankAlign::Bottom => prev_y + max_h - h / 2.0,
					RankAlign::Center => prev_y + max_h / 2.0,
				});
			}
		}
		prev_y += max_h + rank_sep;
	}
}

// ---------- Brandes-Kopf -------------------------------------------------

/// Dense map keyed by [`NodeIdx`]. Node ids are contiguous indices, so the
/// per-node state the BK passes carry is a flat `Vec` rather than a hash map -
/// this is the hot storage of the position phase.
#[derive(Debug, Clone)]
pub struct NodeMap<T> {
	slots: Vec<Option<T>>,
	len: usize,
}

impl<T> Default for NodeMap<T> {
	fn default() -> Self {
		Self {
			slots: Vec::new(),
			len: 0,
		}
	}
}

impl<T> NodeMap<T> {
	#[must_use]
	pub fn with_bound(bound: usize) -> Self {
		let mut slots = Vec::new();
		slots.resize_with(bound, || None);
		Self { slots, len: 0 }
	}

	pub fn get(&self, v: NodeIdx) -> Option<&T> {
		self.slots.get(v.index())?.as_ref()
	}

	pub fn insert(&mut self, v: NodeIdx, value: T) {
		if self.slots.len() <= v.index() {
			self.slots
				.resize_with(v.index() + 1, || None);
		}
		if self.slots[v.index()]
			.replace(value)
			.is_none()
		{
			self.len += 1;
		}
	}

	pub fn contains_key(&self, v: NodeIdx) -> bool {
		self.get(v).is_some()
	}

	pub const fn len(&self) -> usize {
		self.len
	}

	pub const fn is_empty(&self) -> bool {
		self.len == 0
	}

	pub fn iter(&self) -> impl Iterator<Item = (NodeIdx, &T)> + '_ {
		self.slots
			.iter()
			.enumerate()
			.filter_map(|(i, slot)| {
				Some((
					NodeIdx(u32::try_from(i).unwrap_or(u32::MAX)),
					slot.as_ref()?,
				))
			})
	}

	pub fn keys(&self) -> impl Iterator<Item = NodeIdx> + '_ {
		self.iter().map(|(v, _)| v)
	}

	pub fn values(&self) -> impl Iterator<Item = &T> + '_ {
		self.slots
			.iter()
			.filter_map(Option::as_ref)
	}

	pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
		self.slots
			.iter_mut()
			.filter_map(Option::as_mut)
	}
}

impl<T: Copy> NodeMap<T> {
	pub fn get_copied(&self, v: NodeIdx) -> Option<T> {
		self.get(v).copied()
	}
}

/// Compares live entries only, so maps sized to different bounds still match.
impl<T: PartialEq> PartialEq for NodeMap<T> {
	fn eq(&self, other: &Self) -> bool {
		self.len == other.len
			&& self
				.iter()
				.all(|(v, x)| other.get(v) == Some(x))
	}
}

impl<T> std::ops::Index<NodeIdx> for NodeMap<T> {
	type Output = T;
	fn index(&self, v: NodeIdx) -> &T {
		self.get(v)
			.unwrap_or_else(|| panic!("no entry for {v}"))
	}
}

impl<T> FromIterator<(NodeIdx, T)> for NodeMap<T> {
	fn from_iter<I: IntoIterator<Item = (NodeIdx, T)>>(iter: I) -> Self {
		let mut out = Self::default();
		for (v, x) in iter {
			out.insert(v, x);
		}
		out
	}
}

/// Unordered pairs of nodes that must not be aligned with each other. The
/// pair is canonicalised on insert, so lookup order does not matter.
pub type Conflicts = FxHashSet<(NodeIdx, NodeIdx)>;
pub type PositionMap = NodeMap<f64>;
pub type AlignmentResult = (NodeMap<NodeIdx>, NodeMap<NodeIdx>);

/// Public BK API for unit tests. Re-exports of the internal helpers.
pub mod bk {
	pub use super::{
		AlignmentResult,
		Conflicts,
		NodeMap,
		PositionMap,
		add_conflict,
		align_coordinates,
		balance,
		find_smallest_width_alignment,
		find_type1_conflicts,
		find_type2_conflicts,
		has_conflict,
		horizontal_compaction,
		position_x,
		vertical_alignment,
	};
}

pub fn add_conflict(conflicts: &mut Conflicts, v: NodeIdx, w: NodeIdx) {
	let pair = if v > w { (w, v) } else { (v, w) };
	conflicts.insert(pair);
}

pub fn has_conflict(conflicts: &Conflicts, v: NodeIdx, w: NodeIdx) -> bool {
	let pair = if v > w { (w, v) } else { (v, w) };
	conflicts.contains(&pair)
}

pub fn find_type1_conflicts(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeIdx>],
) -> Conflicts {
	let mut conflicts: Conflicts = FxHashSet::default();
	if layering.is_empty() {
		return conflicts;
	}
	let mut preds: Vec<NodeIdx> = Vec::new();
	let mut prev: Option<&Vec<NodeIdx>> = None;
	for layer in layering {
		if let Some(prev_layer) = prev {
			let mut k0 = 0usize;
			let mut scan_pos = 0usize;
			let prev_len = prev_layer.len();
			let last_node = layer.last().copied();
			for (i, &v) in layer.iter().enumerate() {
				let w_opt = find_other_inner_segment_node(graph, v);
				let k1 = match w_opt {
					Some(w) => graph
						.node(w)
						.and_then(|n| n.order)
						.unwrap_or(0),
					None => prev_len,
				};
				if w_opt.is_some() || last_node == Some(v) {
					for &scan_node in layer.iter().take(i + 1).skip(scan_pos) {
						graph.predecessors_into(scan_node, &mut preds);
						let scan_dummy = graph
							.node(scan_node)
							.is_some_and(|n| n.dummy.is_some());
						for &u in &preds {
							// Read the two fields we need; cloning the whole
							// NodeLabel here is expensive (it owns a Vec and
							// a boxed EdgeLabel).
							let Some(u_node) = graph.node(u) else {
								continue;
							};
							let u_pos = u_node.order.unwrap_or(0);
							let u_dummy = u_node.dummy.is_some();
							if (u_pos < k0 || k1 < u_pos)
								&& !(u_dummy && scan_dummy)
							{
								add_conflict(&mut conflicts, u, scan_node);
							}
						}
					}
					scan_pos = i + 1;
					k0 = k1;
				}
			}
		}
		prev = Some(layer);
	}
	conflicts
}

pub fn find_type2_conflicts(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeIdx>],
) -> Conflicts {
	let mut conflicts: Conflicts = FxHashSet::default();
	if layering.is_empty() {
		return conflicts;
	}

	fn scan(
		graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		conflicts: &mut Conflicts,
		south: &[NodeIdx],
		south_pos: usize,
		south_end: usize,
		prev_north_border: i64,
		next_north_border: i64,
	) {
		for i in south_pos..south_end {
			let Some(&v) = south.get(i) else { continue };
			if graph
				.node(v)
				.is_some_and(|n| n.dummy.is_some())
				&& let Some(preds) = graph.predecessors(v)
			{
				for u in preds {
					let u_node = graph
						.node(u)
						.cloned()
						.unwrap_or_default();
					if u_node.dummy.is_some() {
						let uo = u_node.order.unwrap_or(0) as i64;
						if uo < prev_north_border || uo > next_north_border {
							add_conflict(conflicts, u, v);
						}
					}
				}
			}
		}
	}

	let mut prev: Option<&Vec<NodeIdx>> = None;
	for south in layering {
		if let Some(north) = prev {
			let mut prev_north_pos: i64 = -1;
			let mut next_north_pos: i64 = -1;
			let mut south_pos = 0usize;
			for (south_lookahead, &v) in south.iter().enumerate() {
				if graph
					.node(v)
					.and_then(|n| n.dummy)
					.is_some_and(|d| d == Dummy::Border)
				{
					let predecessors = graph
						.predecessors(v)
						.unwrap_or_default();
					if !predecessors.is_empty() {
						let first_pred = predecessors[0];
						next_north_pos = graph
							.node(first_pred)
							.and_then(|n| n.order)
							.unwrap_or(0) as i64;
						scan(
							graph,
							&mut conflicts,
							south,
							south_pos,
							south_lookahead,
							prev_north_pos,
							next_north_pos,
						);
						south_pos = south_lookahead;
						prev_north_pos = next_north_pos;
					}
				}
				scan(
					graph,
					&mut conflicts,
					south,
					south_pos,
					south.len(),
					next_north_pos,
					north.len() as i64,
				);
			}
		}
		prev = Some(south);
	}
	conflicts
}

fn find_other_inner_segment_node(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	v: NodeIdx,
) -> Option<NodeIdx> {
	if graph
		.node(v)
		.is_some_and(|n| n.dummy.is_some())
		&& let Some(preds) = graph.predecessors(v)
	{
		return preds.into_iter().find(|&u| {
			graph
				.node(u)
				.is_some_and(|n| n.dummy.is_some())
		});
	}
	None
}

/// `neighbor_fn` fills the supplied buffer rather than returning a `Vec`:
/// this runs once per node per orientation, so allocating per call showed up
/// as a large share of the position phase.
pub fn vertical_alignment<F>(
	layering: &[Vec<NodeIdx>],
	conflicts: &Conflicts,
	neighbor_fn: F,
) -> AlignmentResult
where
	F: Fn(NodeIdx, &mut Vec<NodeIdx>),
{
	let bound = layering
		.iter()
		.flatten()
		.map(|v| v.index() + 1)
		.max()
		.unwrap_or(0);
	let mut root: NodeMap<NodeIdx> = NodeMap::with_bound(bound);
	let mut align: NodeMap<NodeIdx> = NodeMap::with_bound(bound);
	let mut pos: Vec<usize> = vec![0; bound];

	for layer in layering {
		for (order, &v) in layer.iter().enumerate() {
			root.insert(v, v);
			align.insert(v, v);
			pos[v.index()] = order;
		}
	}

	let mut ws: Vec<NodeIdx> = Vec::new();
	for layer in layering {
		let mut prev_idx: i64 = -1;
		for &v in layer {
			neighbor_fn(v, &mut ws);
			if !ws.is_empty() {
				ws.sort_by_key(|w| pos.get(w.index()).copied().unwrap_or(0));
				let mp = (ws.len() as f64 - 1.0) / 2.0;
				let lo = mp.floor() as usize;
				let hi = mp.ceil() as usize;
				for i in lo..=hi {
					let Some(&w) = ws.get(i) else { continue };
					let pos_w = pos.get(w.index()).copied().unwrap_or(0) as i64;
					if align.get(v) == Some(&v)
						&& prev_idx < pos_w
						&& !has_conflict(conflicts, v, w)
					{
						align.insert(w, v);
						let root_w = root.get_copied(w).unwrap_or(w);
						align.insert(v, root_w);
						root.insert(v, root_w);
						prev_idx = pos_w;
					}
				}
			}
		}
	}

	(root, align)
}

// TODO: Refactor
#[expect(clippy::too_many_lines)]
pub fn horizontal_compaction(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeIdx>],
	root: &NodeMap<NodeIdx>,
	align: &NodeMap<NodeIdx>,
	reverse_sep: bool,
) -> PositionMap {
	let mut xs: PositionMap = PositionMap::with_bound(graph.node_bound());
	let block_g = build_block_graph(graph, layering, root, reverse_sep);
	let border_type = if reverse_sep {
		BorderType::BorderLeft
	} else {
		BorderType::BorderRight
	};

	fn iterate<F1, F2>(
		block_g: &Graph<(), (), f64>,
		mut set_xs: F1,
		next_nodes: F2,
	) where
		F1: FnMut(NodeIdx),
		F2: Fn(NodeIdx) -> Vec<NodeIdx>,
	{
		// Tri-state iterative post-order DFS: WHITE -> GRAY -> BLACK. The
		// original JS dagre uses a two-state visited set, but that re-runs
		// set_xs every time a finalised node is popped from another path -
		// quadratic for the long dummy chains normalise produces. Tracking
		// BLACK explicitly keeps each node's set_xs to exactly one call while
		// preserving the post-order in which dependencies are computed first.
		const WHITE: u8 = 0;
		const GRAY: u8 = 1;
		const BLACK: u8 = 2;
		let nodes = block_g.nodes();
		let mut state: Vec<u8> = vec![WHITE; block_g.node_bound()];
		let mut stack: Vec<NodeIdx> = nodes;
		while let Some(elem) = stack.pop() {
			match state[elem.index()] {
				WHITE => {
					state[elem.index()] = GRAY;
					stack.push(elem);
					for n in next_nodes(elem) {
						if state[n.index()] == WHITE {
							stack.push(n);
						}
					}
				}
				GRAY => {
					set_xs(elem);
					state[elem.index()] = BLACK;
				}
				_ => {}
			}
		}
	}

	let block_g_ref = &block_g;
	let xs_ref = &mut xs;

	// Pass 1: smallest coordinates (uses inEdges).
	{
		let preds = |e: NodeIdx| {
			block_g_ref
				.predecessors(e)
				.unwrap_or_default()
		};
		let mut pass1 = |elem: NodeIdx| {
			let mut max = 0.0_f64;
			let mut any = false;
			for &ei in block_g_ref.in_edge_idxs(elem) {
				let Some((e, &w)) = block_g_ref.edge_entry(ei) else {
					continue;
				};
				any = true;
				let cand = xs_ref.get_copied(e.v).unwrap_or(0.0) + w;
				if cand > max {
					max = cand;
				}
			}
			if any {
				xs_ref.insert(elem, max);
			} else {
				xs_ref.insert(elem, 0.0);
			}
		};
		iterate(block_g_ref, &mut pass1, preds);
	};

	// Pass 2: greatest coordinates (uses outEdges).
	{
		let succs = |e: NodeIdx| {
			block_g_ref
				.successors(e)
				.unwrap_or_default()
		};
		let mut pass2 = |elem: NodeIdx| {
			let mut min = f64::INFINITY;
			for &ei in block_g_ref.out_edge_idxs(elem) {
				let Some((e, &w)) = block_g_ref.edge_entry(ei) else {
					continue;
				};
				let cand = xs_ref.get_copied(e.w).unwrap_or(0.0) - w;
				if cand < min {
					min = cand;
				}
			}
			let bt = graph
				.node(elem)
				.and_then(|n| n.border_type);
			if min.is_finite() && bt != Some(border_type) {
				let cur = xs_ref.get_copied(elem).unwrap_or(0.0);
				xs_ref.insert(elem, cur.max(min));
			}
		};
		iterate(block_g_ref, &mut pass2, succs);
	};

	// Propagate root x to all aligned nodes. The snapshot is needed because
	// a block's root may itself be re-pointed during this loop.
	let xs_root = xs.clone();
	for v in align.keys() {
		if let Some(&rv) = root.get(v)
			&& let Some(&x) = xs_root.get(rv)
		{
			xs.insert(v, x);
		}
	}
	xs
}

fn build_block_graph(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	layering: &[Vec<NodeIdx>],
	root: &NodeMap<NodeIdx>,
	reverse_sep: bool,
) -> Graph<(), (), f64> {
	// Shares the layout graph's node index space, so a block is addressed by
	// its root node's index.
	let mut block_graph: Graph<(), (), f64> = Graph::with_opts(GraphOpts {
		directed: true,
		multigraph: false,
		compound: false,
	});
	block_graph.reserve_nodes(graph.node_bound());
	let gl = graph
		.graph()
		.cloned()
		.unwrap_or_default();
	let node_sep = gl.nodesep.unwrap_or(50.0);
	let edge_sep = gl.edgesep.unwrap_or(20.0);
	let sep_fn =
		move |g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
		      v: NodeIdx,
		      u: NodeIdx|
		      -> f64 { sep(node_sep, edge_sep, reverse_sep, g, v, u) };
	for layer in layering {
		let mut u: Option<NodeIdx> = None;
		for &v in layer {
			let Some(&v_root) = root.get(v) else {
				continue;
			};
			if !block_graph.has_node(v_root) {
				block_graph.set_node(v_root, ());
			}
			if let Some(uu) = u
				&& let Some(&u_root) = root.get(uu)
			{
				let prev_max = block_graph
					.edge(u_root, v_root)
					.copied()
					.unwrap_or(0.0);
				let s = sep_fn(graph, v, uu);
				block_graph.set_edge(u_root, v_root, s.max(prev_max));
			}
			u = Some(v);
		}
	}
	block_graph
}

fn sep(
	node_sep: f64,
	edge_sep: f64,
	reverse_sep: bool,
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	v: NodeIdx,
	w: NodeIdx,
) -> f64 {
	// Called O(N) times per orientation x 4 orientations x 2 passes - cloning
	// NodeLabel here was a measurable cost (Option<Vec<_>> + Option<Box>
	// inside). Read fields by reference instead.
	let v_label = g.node(v);
	let w_label = g.node(w);
	let v_width = v_label.map_or(0., |n| n.width);
	let w_width = w_label.map_or(0., |n| n.width);
	let v_labelpos = v_label.and_then(|n| n.labelpos);
	let w_labelpos = w_label.and_then(|n| n.labelpos);
	let v_is_dummy = v_label.is_some_and(|n| n.dummy.is_some());
	let w_is_dummy = w_label.is_some_and(|n| n.dummy.is_some());

	let mut sum = 0.0;
	let mut delta: Option<f64> = None;

	sum += v_width / 2.0;
	if let Some(lp) = v_labelpos {
		delta = match lp {
			LabelPos::L => Some(-v_width / 2.0),
			LabelPos::R => Some(v_width / 2.0),
			LabelPos::C => None,
		};
	}
	if let Some(d) = delta {
		sum += if reverse_sep { d } else { -d };
	}
	delta = None;
	sum += if v_is_dummy { edge_sep } else { node_sep } / 2.0;
	sum += if w_is_dummy { edge_sep } else { node_sep } / 2.0;
	sum += w_width / 2.0;
	if let Some(lp) = w_labelpos {
		delta = match lp {
			LabelPos::L => Some(w_width / 2.0),
			LabelPos::R => Some(-w_width / 2.0),
			LabelPos::C => None,
		};
	}
	if let Some(d) = delta {
		sum += if reverse_sep { d } else { -d };
	}
	sum
}

pub fn position_x(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
) -> PositionMap {
	#[cfg(feature = "profile")]
	let t0 = std::time::Instant::now();
	let layering = util::build_layer_matrix(graph);
	#[cfg(feature = "profile")]
	let dt_layer = t0.elapsed();
	#[cfg(feature = "profile")]
	let t1 = std::time::Instant::now();
	let mut conflicts = find_type1_conflicts(graph, &layering);
	#[cfg(feature = "profile")]
	let dt_conf = t1.elapsed();
	// findType2Conflicts is mainly relevant for the nesting/border case we
	// skip; we leave it out to keep the port small.
	let _ = &mut conflicts;

	#[cfg(feature = "profile")]
	let (mut t_va, mut t_hc) = (0u128, 0u128);
	let mut xss: FxHashMap<String, PositionMap> = FxHashMap::default();
	for vert in ["u", "d"] {
		let mut adjusted: Vec<Vec<NodeIdx>> = layering.clone();
		if vert == "d" {
			adjusted.reverse();
		}
		for horiz in ["l", "r"] {
			if horiz == "r" {
				for inner in &mut adjusted {
					inner.reverse();
				}
			}
			let neighbor_fn = |v: NodeIdx, out: &mut Vec<NodeIdx>| {
				if vert == "u" {
					graph.predecessors_into(v, out);
				} else {
					graph.successors_into(v, out);
				}
			};
			#[cfg(feature = "profile")]
			let tx = std::time::Instant::now();
			let (root, align) =
				vertical_alignment(&adjusted, &conflicts, neighbor_fn);
			#[cfg(feature = "profile")]
			{
				t_va += tx.elapsed().as_nanos();
			};
			#[cfg(feature = "profile")]
			let ty = std::time::Instant::now();
			let mut xs = horizontal_compaction(
				graph,
				&adjusted,
				&root,
				&align,
				horiz == "r",
			);
			#[cfg(feature = "profile")]
			{
				t_hc += ty.elapsed().as_nanos();
			};
			if horiz == "r" {
				for v in xs.values_mut() {
					*v = -*v;
				}
				// Undo reversal of adjusted for next iter.
				for inner in &mut adjusted {
					inner.reverse();
				}
			}
			xss.insert(format!("{vert}{horiz}"), xs);
		}
	}

	#[cfg(feature = "profile")]
	eprintln!(
		"[dagre]   position_x layer={:.1}ms conflicts={:.1}ms vertical_align={:.1}ms horiz_compact={:.1}ms",
		dt_layer.as_secs_f64() * 1000.0,
		dt_conf.as_secs_f64() * 1000.0,
		t_va as f64 / 1e6,
		t_hc as f64 / 1e6,
	);
	let smallest = find_smallest_width_alignment(graph, &xss);
	align_coordinates(&mut xss, &smallest);
	balance(&xss, graph.graph().and_then(|g| g.align))
}

pub fn find_smallest_width_alignment<S: BuildHasher>(
	graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	xss: &collections::HashMap<String, PositionMap, S>,
) -> PositionMap {
	let mut best: (f64, Option<PositionMap>) = (f64::INFINITY, None);
	for xs in xss.values() {
		let mut min = f64::INFINITY;
		let mut max = f64::NEG_INFINITY;
		for (v, x) in xs.iter() {
			let hw = graph.node(v).map_or(0.0, |n| n.width) / 2.0;
			max = max.max(x + hw);
			min = min.min(x - hw);
		}
		let width = max - min;
		if width < best.0 {
			best = (width, Some(xs.clone()));
		}
	}
	best.1.unwrap_or_default()
}

pub fn align_coordinates<S: BuildHasher>(
	xss: &mut collections::HashMap<String, PositionMap, S>,
	align_to: &PositionMap,
) {
	let align_to_min = align_to
		.values()
		.copied()
		.fold(f64::INFINITY, f64::min);
	let align_to_max = align_to
		.values()
		.copied()
		.fold(f64::NEG_INFINITY, f64::max);
	for vert in ["u", "d"] {
		for horiz in ["l", "r"] {
			let key = format!("{vert}{horiz}");
			let xs = match xss.get(&key) {
				Some(m) => m.clone(),
				None => continue,
			};
			// Skip if this IS the alignTo map.
			let is_align_to = xs.len() == align_to.len()
				&& xs
					.iter()
					.all(|(k, v)| align_to.get(k) == Some(v));
			if is_align_to {
				continue;
			}
			let xs_min = xs
				.values()
				.copied()
				.fold(f64::INFINITY, f64::min);
			let xs_max = xs
				.values()
				.copied()
				.fold(f64::NEG_INFINITY, f64::max);
			let delta = if horiz == "l" {
				align_to_min - xs_min
			} else {
				align_to_max - xs_max
			};
			if delta != 0.0 && delta.is_finite() {
				let shifted: PositionMap = xs
					.iter()
					.map(|(k, v)| (k, v + delta))
					.collect();
				xss.insert(key, shifted);
			}
		}
	}
}

pub fn balance<S: BuildHasher>(
	xss: &collections::HashMap<String, PositionMap, S>,
	align: Option<Align>,
) -> PositionMap {
	let Some(ul) = xss.get("ul") else {
		return PositionMap::default();
	};
	let mut out = PositionMap::default();
	for v in ul.keys() {
		if let Some(a) = align
			&& let Some(m) = xss.get(a.to_str())
			&& let Some(&x) = m.get(v)
		{
			out.insert(v, x);
			continue;
		}
		let mut xs: Vec<f64> = xss
			.values()
			.filter_map(|m| m.get_copied(v))
			.collect();
		xs.sort_by(|a, b| {
			a.partial_cmp(b)
				.unwrap_or(Ordering::Equal)
		});
		let a = xs.get(1).copied().unwrap_or(0.0);
		let b = xs.get(2).copied().unwrap_or(0.0);
		out.insert(v, f64::midpoint(a, b));
	}
	out
}
