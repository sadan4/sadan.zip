//! Port of the @dagrejs/graphlib `Graph` class, restricted to the surface
//! actually used by the dagre layout pipeline.
//!
//! The original graph is generic over three label types: graph label `G`,
//! node label `N`, and edge label `E`. We keep that shape.
//!
//! Supports:
//!   - directed graphs (the only mode dagre actually uses)
//!   - multigraph (named edges)
//!   - compound graphs (parent / children)
//!
//! Edge identity is the triple `(v, w, name)`. For non-multigraphs the name is
//! always the empty string.

use std::collections::{self, BTreeMap, BTreeSet, HashMap, HashSet};

pub use smol_str::SmolStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Identifier type for nodes and edges. [`SmolStr`] inlines strings up to 23 bytes;
/// dagre node/edge ids in practice (numeric module ids, "a"/"b", "rev1", "_d24")
/// are well under that, so the vast majority never heap-allocate.
pub type NodeId = SmolStr;

/// Identifies an edge by its endpoints and (for multigraphs) a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Edge {
	pub v: NodeId,
	pub w: NodeId,
	pub name: Option<NodeId>,
}

impl Edge {
	#[must_use]
	pub fn new(v: impl Into<NodeId>, w: impl Into<NodeId>) -> Self {
		Self {
			v: v.into(),
			w: w.into(),
			name: None,
		}
	}
	#[must_use]
	pub fn with_name(
		v: impl Into<NodeId>,
		w: impl Into<NodeId>,
		name: impl Into<NodeId>,
	) -> Self {
		Self {
			v: v.into(),
			w: w.into(),
			name: Some(name.into()),
		}
	}
}

/// Graph configuration. Default is a directed simple graph (matches graphlib).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GraphOpts {
	pub directed: bool,
	pub multigraph: bool,
	pub compound: bool,
}

impl Default for GraphOpts {
	fn default() -> Self {
		Self {
			directed: true,
			multigraph: false,
			compound: false,
		}
	}
}

impl GraphOpts {
	#[must_use]
	pub fn directed() -> Self {
		Self::default()
	}
	#[must_use]
	pub const fn undirected() -> Self {
		Self {
			directed: false,
			multigraph: false,
			compound: false,
		}
	}
	#[must_use]
	pub const fn multigraph(mut self) -> Self {
		self.multigraph = true;
		self
	}
	#[must_use]
	pub const fn compound(mut self) -> Self {
		self.compound = true;
		self
	}
}

type NodeLabelFactory<N> = Box<dyn Fn(&str) -> N>;
type EdgeLabelFactory<E> = Box<dyn Fn(&Edge) -> E>;

/// The graph itself. `G`, `N`, `E` are the label types.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
	feature = "serde",
	serde(bound(
		serialize = "G: Serialize, N: Serialize, E: Serialize",
		deserialize = "G: Deserialize<'de>, N: Deserialize<'de>, E: Deserialize<'de>"
	))
)]
pub struct Graph<G, N, E> {
	is_directed: bool,
	is_multigraph: bool,
	is_compound: bool,

	label: Option<G>,

	// Default label factory used by `setNode(v)` (without label).
	#[cfg_attr(feature = "serde", serde(skip))]
	default_node_label: Option<NodeLabelFactory<N>>,
	#[cfg_attr(feature = "serde", serde(skip))]
	default_edge_label: Option<EdgeLabelFactory<E>>,

	/// Insertion-ordered node ids.
	node_order: Vec<NodeId>,
	node_index: HashMap<NodeId, usize>,
	node_labels: HashMap<NodeId, N>,

	/// Compound graph: parent of each node.
	parent: HashMap<NodeId, NodeId>,
	/// Children of each parent ("\x00" key = root children, matching JS impl).
	children: HashMap<NodeId, BTreeSet<NodeId>>,

	/// out[v] -> { `edge_id` -> Edge }
	out_edges: HashMap<NodeId, BTreeMap<NodeId, Edge>>,
	/// in[v] -> { `edge_id` -> Edge }
	in_edges: HashMap<NodeId, BTreeMap<NodeId, Edge>>,
	/// predecessor count: preds[v][u] = number of u->v edges
	preds: HashMap<NodeId, HashMap<NodeId, usize>>,
	sucs: HashMap<NodeId, HashMap<NodeId, usize>>,

	/// Insertion-ordered edge ids -> Edge.
	edge_order: Vec<NodeId>,
	edge_index: HashMap<NodeId, usize>,
	/// Edge label, keyed by `edge_id`.
	edge_labels: HashMap<NodeId, E>,
	/// Edge object, keyed by `edge_id`.
	edge_objs: HashMap<NodeId, Edge>,
}

const GRAPH_NODE: &str = "\x00";

impl<G, N, E> Graph<G, N, E> {
	pub fn new() -> Self {
		Self::with_opts(GraphOpts::directed())
	}

	pub fn with_opts(opts: GraphOpts) -> Self {
		let is_directed = opts.directed;
		let mut g = Self {
			is_directed,
			is_multigraph: opts.multigraph,
			is_compound: opts.compound,
			label: None,
			default_node_label: None,
			default_edge_label: None,
			node_order: Vec::new(),
			node_index: HashMap::new(),
			node_labels: HashMap::new(),
			parent: HashMap::new(),
			children: HashMap::new(),
			out_edges: HashMap::new(),
			in_edges: HashMap::new(),
			preds: HashMap::new(),
			sucs: HashMap::new(),
			edge_order: Vec::new(),
			edge_index: HashMap::new(),
			edge_labels: HashMap::new(),
			edge_objs: HashMap::new(),
		};
		if g.is_compound {
			g.children
				.insert(GRAPH_NODE.into(), BTreeSet::new());
		}
		g
	}

	pub const fn is_directed(&self) -> bool {
		self.is_directed
	}
	pub const fn is_multigraph(&self) -> bool {
		self.is_multigraph
	}
	/// Promote a simple graph to a multigraph in place. Existing edges keep
	/// their identity (their `name` field is already `None`, which is also the
	/// canonical id for the multigraph version of the same edge), so no edges
	/// move or merge. Used by layout to satisfy its internal requirement that
	/// named dummy / reversed edges can be inserted.
	pub const fn set_multigraph(&mut self, multigraph: bool) {
		self.is_multigraph = multigraph;
	}
	pub const fn is_compound(&self) -> bool {
		self.is_compound
	}

	// ---- graph label ----------------------------------------------------

	pub fn set_graph(&mut self, label: G) -> &mut Self {
		self.label = Some(label);
		self
	}
	pub const fn graph(&self) -> Option<&G> {
		self.label.as_ref()
	}
	pub const fn graph_mut(&mut self) -> Option<&mut G> {
		self.label.as_mut()
	}

	pub fn set_default_node_label<F: Fn(&str) -> N + 'static>(
		&mut self,
		f: F,
	) -> &mut Self {
		self.default_node_label = Some(Box::new(f));
		self
	}
	pub fn set_default_edge_label<F: Fn(&Edge) -> E + 'static>(
		&mut self,
		f: F,
	) -> &mut Self {
		self.default_edge_label = Some(Box::new(f));
		self
	}

	// ---- nodes ----------------------------------------------------------

	pub const fn node_count(&self) -> usize {
		self.node_order.len()
	}
	pub fn nodes(&self) -> Vec<NodeId> {
		self.node_order.clone()
	}
	/// Borrowing alternative to `nodes()` for hot loops. Avoids cloning the
	/// 60k-entry `node_order` Vec when callers only need to iterate.
	pub fn nodes_iter(&self) -> impl Iterator<Item = &str> {
		self.node_order
			.iter()
			.map(SmolStr::as_str)
	}
	pub fn has_node(&self, v: &str) -> bool {
		self.node_labels.contains_key(v)
	}

	pub fn set_node(&mut self, v: impl Into<NodeId>, label: N) -> &mut Self {
		let v = v.into();
		if let collections::hash_map::Entry::Occupied(mut e) =
			self.node_labels.entry(v.clone())
		{
			e.insert(label);
			return self;
		}
		self.node_labels
			.insert(v.clone(), label);
		self.node_index
			.insert(v.clone(), self.node_order.len());
		self.node_order.push(v.clone());
		self.in_edges
			.insert(v.clone(), BTreeMap::new());
		self.out_edges
			.insert(v.clone(), BTreeMap::new());
		self.preds
			.insert(v.clone(), HashMap::new());
		self.sucs
			.insert(v.clone(), HashMap::new());
		if self.is_compound {
			self.parent
				.insert(v.clone(), GRAPH_NODE.into());
			self.children
				.entry(GRAPH_NODE.into())
				.or_default()
				.insert(v.clone());
			self.children
				.insert(v, BTreeSet::new());
		}
		self
	}

	/// Like `setNode(v)` in JS with no label: uses default label factory if
	/// one was registered, else falls back to `N::default()`.
	pub fn set_node_default(&mut self, v: impl Into<NodeId>) -> &mut Self
	where
		N: Default,
	{
		let v = v.into();
		if self.node_labels.contains_key(&v) {
			return self;
		}
		let label = match &self.default_node_label {
			Some(f) => f(&v),
			None => N::default(),
		};
		self.set_node(v, label)
	}

	pub fn node(&self, v: &str) -> Option<&N> {
		self.node_labels.get(v)
	}
	pub fn node_mut(&mut self, v: &str) -> Option<&mut N> {
		self.node_labels.get_mut(v)
	}

	pub fn remove_node(&mut self, v: &str) {
		if !self.has_node(v) {
			return;
		}
		// Remove from compound parent/children
		if self.is_compound {
			// Detach children up to root.
			let cs: Vec<NodeId> = self
				.children
				.get(v)
				.map(|s| s.iter().cloned().collect())
				.unwrap_or_default();
			// Remove this v from its parent's child set.
			if let Some(p) = self.parent.remove(v)
				&& let Some(set) = self.children.get_mut(&p)
			{
				set.remove(v);
			}
			for c in &cs {
				// children become root-level.
				self.parent
					.insert(c.clone(), GRAPH_NODE.into());
				self.children
					.entry(GRAPH_NODE.into())
					.or_default()
					.insert(c.clone());
			}
			self.children.remove(v);
		}

		// Remove incident edges.
		let in_es: Vec<Edge> = self
			.in_edges
			.get(v)
			.map(|m| m.values().cloned().collect())
			.unwrap_or_default();
		let out_es: Vec<Edge> = self
			.out_edges
			.get(v)
			.map(|m| m.values().cloned().collect())
			.unwrap_or_default();
		for e in in_es.into_iter().chain(out_es) {
			self.remove_edge_obj(&e);
		}

		self.node_labels.remove(v);
		self.in_edges.remove(v);
		self.out_edges.remove(v);
		self.preds.remove(v);
		self.sucs.remove(v);
		// O(1) removal via swap_remove. normalize::undo deletes ~tens of
		// thousands of dummies, so any O(N) work here turns the phase into
		// O(N²) — measured at 15s on a 63k-node graph before this change.
		// Iteration order at the swapped slot changes, but no caller in this
		// crate relies on insertion-order stability after a removal: layer
		// algorithms sort by node.rank/order, not by nodes() position.
		if let Some(&idx) = self.node_index.get(v) {
			self.node_order.swap_remove(idx);
			self.node_index.remove(v);
			if let Some(swapped) = self.node_order.get(idx) {
				self.node_index
					.insert(swapped.clone(), idx);
			}
		}
	}

	// ---- compound -------------------------------------------------------

	pub fn set_parent(&mut self, v: &str, parent: Option<&str>)
	where
		N: Default,
	{
		if !self.is_compound {
			panic!("Cannot set parent in a non-compound graph");
		}
		if !self.has_node(v) {
			self.set_node_default(v.to_string());
		}
		let new_parent: NodeId = parent.unwrap_or(GRAPH_NODE).into();
		// Disallow cycles.
		if parent.is_some() {
			let mut ancestor = Some(new_parent.clone());
			while let Some(a) = ancestor {
				if a == v {
					panic!(
						"Setting {new_parent} as parent of {v} would create a cycle",
					);
				}
				ancestor = self.parent.get(&a).cloned();
				if ancestor.as_deref() == Some(GRAPH_NODE) {
					ancestor = None;
				}
			}
		}
		if let Some(parent) = parent
			&& !self.has_node(parent)
		{
			self.set_node_default(parent.to_string());
		}
		// Detach from previous parent.
		if let Some(prev) = self.parent.get(v).cloned()
			&& let Some(set) = self.children.get_mut(&prev)
		{
			set.remove(v);
		}
		self.parent
			.insert(v.into(), new_parent.clone());
		self.children
			.entry(new_parent)
			.or_default()
			.insert(v.into());
	}

	pub fn parent(&self, v: &str) -> Option<&str> {
		if !self.is_compound {
			return None;
		}
		let p = self.parent.get(v)?;
		if p == GRAPH_NODE {
			None
		} else {
			Some(p.as_str())
		}
	}

	/// Returns children of `v`, or root-level nodes if `v` is None.
	pub fn children(&self, v: Option<&str>) -> Vec<NodeId> {
		let key = v.unwrap_or(GRAPH_NODE);
		if self.is_compound {
			self.children
				.get(key)
				.map(|s| s.iter().cloned().collect())
				.unwrap_or_default()
		} else if v.is_none() {
			self.nodes()
		} else {
			vec![]
		}
	}

	// ---- edges ----------------------------------------------------------

	pub const fn edge_count(&self) -> usize {
		self.edge_order.len()
	}

	pub fn edges(&self) -> Vec<Edge> {
		self.edge_order
			.iter()
			.map(|id| self.edge_objs.get(id).cloned().unwrap())
			.collect()
	}

	pub fn edges_iter(&self) -> impl Iterator<Item = &Edge> {
		self.edge_order
			.iter()
			.map(|id| self.edge_objs.get(id).unwrap())
	}

	fn edge_id(&self, v: &str, w: &str, name: Option<&str>) -> NodeId {
		let s = if !self.is_directed && v > w {
			format!("{}\x01{}\x01{}", w, v, name.unwrap_or(""))
		} else {
			format!("{}\x01{}\x01{}", v, w, name.unwrap_or(""))
		};
		s.into()
	}

	pub fn has_edge(&self, v: &str, w: &str) -> bool {
		self.has_edge_named(v, w, None)
	}
	pub fn has_edge_named(&self, v: &str, w: &str, name: Option<&str>) -> bool {
		let id = self.edge_id(v, w, name);
		self.edge_index.contains_key(&id)
	}
	pub fn has_edge_obj(&self, e: &Edge) -> bool {
		self.has_edge_named(&e.v, &e.w, e.name.as_deref())
	}

	pub fn set_edge(
		&mut self,
		v: impl Into<NodeId>,
		w: impl Into<NodeId>,
		label: E,
	) where
		N: Default,
		E: Default,
	{
		self.set_edge_full(v, w, None, Some(label));
	}

	pub fn set_edge_named(
		&mut self,
		v: impl Into<NodeId>,
		w: impl Into<NodeId>,
		label: E,
		name: Option<NodeId>,
	) where
		N: Default,
		E: Default,
	{
		self.set_edge_full(v, w, name, Some(label));
	}

	/// Equivalent to setEdge(v, w) with no label — applies default factory.
	pub fn set_edge_default(
		&mut self,
		v: impl Into<NodeId>,
		w: impl Into<NodeId>,
	) where
		N: Default,
		E: Default,
	{
		self.set_edge_full(v, w, None, None);
	}

	pub fn set_edge_obj(&mut self, e: &Edge, label: E)
	where
		N: Default,
		E: Default,
	{
		self.set_edge_full(
			e.v.clone(),
			e.w.clone(),
			e.name.clone(),
			Some(label),
		);
	}
	pub fn set_edge_obj_default(&mut self, e: &Edge)
	where
		N: Default,
		E: Default,
	{
		self.set_edge_full(e.v.clone(), e.w.clone(), e.name.clone(), None);
	}

	fn set_edge_full(
		&mut self,
		v: impl Into<NodeId>,
		w: impl Into<NodeId>,
		name: Option<NodeId>,
		label: Option<E>,
	) where
		N: Default,
		E: Default,
	{
		let mut v = v.into();
		let mut w = w.into();
		if name.is_some() && !self.is_multigraph {
			panic!("Cannot set a named edge when isMultigraph = false");
		}
		let id = self.edge_id(&v, &w, name.as_deref());

		if self.edge_index.contains_key(&id) {
			if let Some(label) = label {
				self.edge_labels.insert(id, label);
			} else if let Some(f) = &self.default_edge_label {
				let e = Edge {
					v: v.clone(),
					w: w.clone(),
					name,
				};
				let l = f(&e);
				self.edge_labels.insert(id, l);
			}
			return;
		}
		if !self.has_node(&v) {
			self.set_node_default(v.clone());
		}
		if !self.has_node(&w) {
			self.set_node_default(w.clone());
		}
		// Canonicalize for undirected: ensure v <= w.
		if !self.is_directed && v > w {
			std::mem::swap(&mut v, &mut w);
		}
		let edge = Edge {
			v: v.clone(),
			w: w.clone(),
			name,
		};
		let label = match label {
			Some(l) => l,
			None => match &self.default_edge_label {
				Some(f) => f(&edge),
				None => E::default(),
			},
		};

		self.edge_labels
			.insert(id.clone(), label);
		self.edge_objs
			.insert(id.clone(), edge.clone());
		self.edge_index
			.insert(id.clone(), self.edge_order.len());
		self.edge_order.push(id.clone());

		self.out_edges
			.get_mut(&v)
			.unwrap()
			.insert(id.clone(), edge.clone());
		self.in_edges
			.get_mut(&w)
			.unwrap()
			.insert(id, edge);
		*self
			.sucs
			.get_mut(&v)
			.unwrap()
			.entry(w.clone())
			.or_insert(0) += 1;
		*self
			.preds
			.get_mut(&w)
			.unwrap()
			.entry(v.clone())
			.or_insert(0) += 1;
	}

	pub fn edge(&self, v: &str, w: &str) -> Option<&E> {
		self.edge_full(v, w, None)
	}
	pub fn edge_mut(&mut self, v: &str, w: &str) -> Option<&mut E> {
		self.edge_full_mut(v, w, None)
	}
	pub fn edge_full(
		&self,
		v: &str,
		w: &str,
		name: Option<&str>,
	) -> Option<&E> {
		let id = self.edge_id(v, w, name);
		self.edge_labels.get(&id)
	}
	pub fn edge_full_mut(
		&mut self,
		v: &str,
		w: &str,
		name: Option<&str>,
	) -> Option<&mut E> {
		let id = self.edge_id(v, w, name);
		self.edge_labels.get_mut(&id)
	}
	pub fn edge_obj(&self, e: &Edge) -> Option<&E> {
		self.edge_full(&e.v, &e.w, e.name.as_deref())
	}
	pub fn edge_obj_mut(&mut self, e: &Edge) -> Option<&mut E> {
		self.edge_full_mut(&e.v, &e.w, e.name.as_deref())
	}

	pub fn remove_edge(&mut self, v: &str, w: &str) {
		self.remove_edge_named(v, w, None);
	}
	pub fn remove_edge_named(&mut self, v: &str, w: &str, name: Option<&str>) {
		let id = self.edge_id(v, w, name);
		if let Some(e) = self.edge_objs.get(&id).cloned() {
			let (vv, ww) = (e.v.clone(), e.w);
			self.edge_labels.remove(&id);
			self.edge_objs.remove(&id);
			// See remove_node: O(1) swap_remove instead of order-preserving
			// shift. edge_index stays consistent for the swapped element only;
			// no caller relies on stable edges() iteration order after a delete.
			if let Some(&idx) = self.edge_index.get(&id) {
				self.edge_order.swap_remove(idx);
				self.edge_index.remove(&id);
				if let Some(swapped) = self.edge_order.get(idx) {
					self.edge_index
						.insert(swapped.clone(), idx);
				}
			}
			if let Some(m) = self.out_edges.get_mut(&vv) {
				m.remove(&id);
			}
			if let Some(m) = self.in_edges.get_mut(&ww) {
				m.remove(&id);
			}
			if let Some(m) = self.sucs.get_mut(&vv)
				&& let Some(cnt) = m.get_mut(&ww)
			{
				*cnt -= 1;
				if *cnt == 0 {
					m.remove(&ww);
				}
			}
			if let Some(m) = self.preds.get_mut(&ww)
				&& let Some(cnt) = m.get_mut(&vv)
			{
				*cnt -= 1;
				if *cnt == 0 {
					m.remove(&vv);
				}
			}
		}
	}
	pub fn remove_edge_obj(&mut self, e: &Edge) {
		self.remove_edge_named(&e.v, &e.w, e.name.as_deref());
	}

	// ---- neighborhood ---------------------------------------------------

	pub fn in_edges(&self, v: &str) -> Option<Vec<Edge>> {
		let m = self.in_edges.get(v)?;
		Some(m.values().cloned().collect())
	}
	pub fn in_edges_iter(
		&self,
		v: &str,
	) -> Option<impl Iterator<Item = &Edge>> {
		Some(self.in_edges.get(v)?.values())
	}
	pub fn in_edges_from(&self, v: &str, u: &str) -> Option<Vec<Edge>> {
		let m = self.in_edges.get(v)?;
		Some(
			m.values()
				.filter(|e| e.v == u)
				.cloned()
				.collect(),
		)
	}
	pub fn out_edges(&self, v: &str) -> Option<Vec<Edge>> {
		let m = self.out_edges.get(v)?;
		Some(m.values().cloned().collect())
	}
	pub fn out_edges_iter(
		&self,
		v: &str,
	) -> Option<impl Iterator<Item = &Edge>> {
		Some(self.out_edges.get(v)?.values())
	}
	pub fn out_edges_to(&self, v: &str, w: &str) -> Option<Vec<Edge>> {
		let m = self.out_edges.get(v)?;
		Some(
			m.values()
				.filter(|e| e.w == w)
				.cloned()
				.collect(),
		)
	}
	pub fn node_edges(&self, v: &str) -> Option<Vec<Edge>> {
		let inv = self.in_edges(v)?;
		let mut out = self.out_edges(v)?;
		out.extend(inv);
		Some(out)
	}
	pub fn predecessors(&self, v: &str) -> Option<Vec<NodeId>> {
		let m = self.preds.get(v)?;
		Some(m.keys().cloned().collect())
	}
	pub fn successors(&self, v: &str) -> Option<Vec<NodeId>> {
		let m = self.sucs.get(v)?;
		Some(m.keys().cloned().collect())
	}
	pub fn neighbors(&self, v: &str) -> Option<Vec<NodeId>> {
		let mut s: Vec<NodeId> = self.predecessors(v)?;
		let succ = self.successors(v)?;
		let set: HashSet<NodeId> = s.iter().cloned().collect();
		for n in succ {
			if !set.contains(&n) {
				s.push(n);
			}
		}
		Some(s)
	}

	pub fn sources(&self) -> Vec<NodeId> {
		self.node_order
			.iter()
			.filter(|v| {
				self.preds
					.get(*v)
					.is_none_or(HashMap::is_empty)
			})
			.cloned()
			.collect()
	}
	/// Convenience: like graphlib's `setPath`, creates a chain of edges.
	/// Each edge gets the default edge label (so `default_edge_label` must
	/// have been set, or the labels are written via the caller).
	pub fn set_path(&mut self, path: &[&str])
	where
		N: Default,
		E: Default,
	{
		for i in 1..path.len() {
			// Use the registered default-edge-label factory if any, else
			// E::default(). This matches graphlib's setPath semantics where
			// edges inherit the default label.
			self.set_edge_full(
				NodeId::from(path[i - 1]),
				NodeId::from(path[i]),
				None,
				None,
			);
		}
	}

	pub fn sinks(&self) -> Vec<NodeId> {
		self.node_order
			.iter()
			.filter(|v| {
				self.sucs
					.get(*v)
					.is_none_or(HashMap::is_empty)
			})
			.cloned()
			.collect()
	}
}

impl<G, N, E> Default for Graph<G, N, E> {
	fn default() -> Self {
		Self::new()
	}
}

// ---- graph algorithms used by network-simplex --------------------------

pub mod alg {
	use super::{Graph, NodeId};
	use std::collections::HashSet;

	/// Postorder DFS traversal — used by network-simplex.
	pub fn postorder<G, N, E>(
		g: &Graph<G, N, E>,
		starts: &[NodeId],
	) -> Vec<NodeId> {
		let mut visited: HashSet<NodeId> = HashSet::new();
		let mut result: Vec<NodeId> = Vec::new();
		for s in starts {
			if !g.has_node(s) {
				panic!("postorder: node {s} not in graph");
			}
			dfs(g, s, &mut visited, &mut result, true);
		}
		result
	}

	/// Tarjan-style strongly connected components on a directed graph; used
	/// by `find_cycles`. Only the SCCs with > 1 node (or self-loops) form
	/// cycles.
	pub fn tarjan<G, N, E>(g: &Graph<G, N, E>) -> Vec<Vec<NodeId>> {
		struct State<'a, G: 'a, N: 'a, E: 'a> {
			g: &'a Graph<G, N, E>,
			index: usize,
			stack: Vec<NodeId>,
			on_stack: std::collections::HashSet<NodeId>,
			indices: std::collections::HashMap<NodeId, usize>,
			lowlinks: std::collections::HashMap<NodeId, usize>,
			results: Vec<Vec<NodeId>>,
		}
		fn strong_connect<G, N, E>(s: &mut State<G, N, E>, v: &str) {
			s.indices.insert(v.into(), s.index);
			s.lowlinks.insert(v.into(), s.index);
			s.index += 1;
			s.stack.push(v.into());
			s.on_stack.insert(v.into());
			for w in s.g.successors(v).unwrap_or_default() {
				if !s.indices.contains_key(&w) {
					strong_connect(s, &w);
					let lw = *s.lowlinks.get(&w).unwrap();
					let lv = *s.lowlinks.get(v).unwrap();
					s.lowlinks.insert(v.into(), lv.min(lw));
				} else if s.on_stack.contains(&w) {
					let iw = *s.indices.get(&w).unwrap();
					let lv = *s.lowlinks.get(v).unwrap();
					s.lowlinks.insert(v.into(), lv.min(iw));
				}
			}
			if s.lowlinks.get(v) == s.indices.get(v) {
				let mut comp = Vec::new();
				while let Some(w) = s.stack.pop() {
					s.on_stack.remove(&w);
					let stop = w == v;
					comp.push(w);
					if stop {
						break;
					}
				}
				s.results.push(comp);
			}
		}
		let mut state = State {
			g,
			index: 0,
			stack: Vec::new(),
			on_stack: std::collections::HashSet::new(),
			indices: std::collections::HashMap::new(),
			lowlinks: std::collections::HashMap::new(),
			results: Vec::new(),
		};
		for v in g.nodes() {
			if !state.indices.contains_key(&v) {
				strong_connect(&mut state, &v);
			}
		}
		state.results
	}

	/// Returns the cycles in the graph: SCCs with more than one node, plus
	/// any self-loop (which are SCCs of size 1 with an edge to themselves).
	pub fn find_cycles<G, N, E>(g: &Graph<G, N, E>) -> Vec<Vec<NodeId>> {
		tarjan(g)
			.into_iter()
			.filter(|comp| {
				let len = comp.len();
				if len == 1 {
					let v = &comp[0];
					g.has_edge(v, v)
				} else {
					len > 1
				}
			})
			.collect()
	}

	/// Preorder DFS traversal.
	pub fn preorder<G, N, E>(
		g: &Graph<G, N, E>,
		starts: &[NodeId],
	) -> Vec<NodeId> {
		let mut visited: HashSet<NodeId> = HashSet::new();
		let mut result: Vec<NodeId> = Vec::new();
		for s in starts {
			if !g.has_node(s) {
				panic!("preorder: node {s} not in graph");
			}
			dfs(g, s, &mut visited, &mut result, false);
		}
		result
	}

	fn dfs<G, N, E>(
		g: &Graph<G, N, E>,
		v: &str,
		visited: &mut HashSet<NodeId>,
		result: &mut Vec<NodeId>,
		postorder: bool,
	) {
		if visited.contains(v) {
			return;
		}
		visited.insert(v.into());
		if !postorder {
			result.push(v.into());
		}
		// For undirected graphs, neighbors; for directed, successors.
		let next = if g.is_directed() {
			g.successors(v).unwrap_or_default()
		} else {
			g.neighbors(v).unwrap_or_default()
		};
		for w in next {
			dfs(g, &w, visited, result, postorder);
		}
		if postorder {
			result.push(v.into());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_node_edge() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		g.set_node("a", 1);
		g.set_node("b", 2);
		g.set_edge("a", "b", 10);
		assert_eq!(g.node_count(), 2);
		assert_eq!(g.edge_count(), 1);
		assert_eq!(g.edge("a", "b"), Some(&10));
		assert_eq!(g.successors("a").unwrap(), vec![NodeId::from("b")]);
		assert_eq!(g.predecessors("b").unwrap(), vec![NodeId::from("a")]);
	}

	#[test]
	fn remove_node_clears_edges() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		g.set_node("a", 1);
		g.set_node("b", 2);
		g.set_edge("a", "b", 10);
		g.remove_node("a");
		assert_eq!(g.edge_count(), 0);
		assert!(!g.has_node("a"));
	}

	#[test]
	fn multigraph_named_edges() {
		let mut g: Graph<(), i32, i32> =
			Graph::with_opts(GraphOpts::directed().multigraph());
		g.set_node("a", 1);
		g.set_node("b", 2);
		g.set_edge_named("a", "b", 1, Some("e1".into()));
		g.set_edge_named("a", "b", 2, Some("e2".into()));
		assert_eq!(g.edge_count(), 2);
	}

	#[cfg(feature = "serde")]
	#[test]
	fn serde_roundtrip() {
		let mut g: Graph<String, i32, i32> = Graph::new();
		g.set_graph("hello".to_string());
		g.set_node("a", 1);
		g.set_node("b", 2);
		g.set_edge("a", "b", 10);
		let json = serde_json::to_string(&g).unwrap();
		let g2: Graph<String, i32, i32> = serde_json::from_str(&json).unwrap();
		assert_eq!(g2.graph(), Some(&"hello".to_string()));
		assert_eq!(g2.node_count(), 2);
		assert_eq!(g2.edge_count(), 1);
		assert_eq!(g2.edge("a", "b"), Some(&10));
		assert_eq!(g2.successors("a").unwrap(), vec![NodeId::from("b")]);
	}

	#[test]
	fn compound_parent_children() {
		let mut g: Graph<(), i32, i32> =
			Graph::with_opts(GraphOpts::directed().compound());
		g.set_node("p", 0);
		g.set_node("c", 1);
		g.set_parent("c", Some("p"));
		assert_eq!(g.parent("c"), Some("p"));
		assert_eq!(g.children(Some("p")), vec![NodeId::from("c")]);
	}
}
