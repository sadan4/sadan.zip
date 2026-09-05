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
//! Nodes and edges are addressed by dense `u32` indices, not strings. Node
//! identity is a [`NodeIdx`]; edge identity is the triple `(v, w, name)`
//! captured by [`Edge`], which is `Copy` and hashes without allocating. A
//! node may optionally carry a human-readable name, stored in a side table
//! that the layout pipeline never touches.

use std::mem;

use rustc_hash::FxHashMap;
pub use smol_str::SmolStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Identifier for a node. Indexes directly into the graph's slot table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct NodeIdx(pub u32);

impl NodeIdx {
	#[must_use]
	pub const fn index(self) -> usize {
		self.0 as usize
	}
}

impl std::fmt::Display for NodeIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "#{}", self.0)
	}
}

/// Identifier for an edge slot. Only used to index the edge table; edge
/// *identity* is [`Edge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct EdgeIdx(pub u32);

impl EdgeIdx {
	#[must_use]
	pub const fn index(self) -> usize {
		self.0 as usize
	}
}

/// Opaque token distinguishing parallel edges between the same two nodes.
/// Minted by [`Graph::fresh_edge_name`]. Nothing inspects its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct EdgeName(pub u32);

/// Identifies an edge by its endpoints and (for multigraphs) a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Edge {
	pub v: NodeIdx,
	pub w: NodeIdx,
	pub name: Option<EdgeName>,
}

impl Edge {
	#[must_use]
	pub const fn new(v: NodeIdx, w: NodeIdx) -> Self {
		Self { v, w, name: None }
	}
	#[must_use]
	pub const fn with_name(v: NodeIdx, w: NodeIdx, name: EdgeName) -> Self {
		Self {
			v,
			w,
			name: Some(name),
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

mod sealed {
	pub trait Sealed {}
	impl Sealed for super::NodeIdx {}
	impl Sealed for &super::NodeIdx {}
	impl Sealed for &str {}
	impl Sealed for String {}
	impl Sealed for &String {}
	impl Sealed for super::SmolStr {}
	impl Sealed for &super::SmolStr {}
}

/// Something that names a node position for *writing*: either an index, or a
/// string that gets interned into the name side-table (creating the node if it
/// is new). Lets callers keep using readable names where ergonomics matter
/// while the layout pipeline stays on raw indices.
pub trait NodeKey<G, N, E>: sealed::Sealed {
	fn resolve(self, g: &mut Graph<G, N, E>) -> NodeIdx;
}

impl<G, N, E> NodeKey<G, N, E> for NodeIdx {
	fn resolve(self, _g: &mut Graph<G, N, E>) -> Self {
		self
	}
}
impl<G, N, E> NodeKey<G, N, E> for &NodeIdx {
	fn resolve(self, _g: &mut Graph<G, N, E>) -> NodeIdx {
		*self
	}
}
impl<G, N: Default, E> NodeKey<G, N, E> for &str {
	fn resolve(self, g: &mut Graph<G, N, E>) -> NodeIdx {
		g.node_named_or_insert(self)
	}
}
impl<G, N: Default, E> NodeKey<G, N, E> for &SmolStr {
	fn resolve(self, g: &mut Graph<G, N, E>) -> NodeIdx {
		g.node_named_or_insert(self)
	}
}
impl<G, N: Default, E> NodeKey<G, N, E> for SmolStr {
	fn resolve(self, g: &mut Graph<G, N, E>) -> NodeIdx {
		g.node_named_or_insert(&self)
	}
}
impl<G, N: Default, E> NodeKey<G, N, E> for String {
	fn resolve(self, g: &mut Graph<G, N, E>) -> NodeIdx {
		g.node_named_or_insert(&self)
	}
}
impl<G, N: Default, E> NodeKey<G, N, E> for &String {
	fn resolve(self, g: &mut Graph<G, N, E>) -> NodeIdx {
		g.node_named_or_insert(self)
	}
}

/// Something that names an *existing* node, for reading. Unknown names
/// resolve to `None` rather than creating anything.
pub trait NodeRef<G, N, E>: sealed::Sealed {
	fn lookup(self, g: &Graph<G, N, E>) -> Option<NodeIdx>;
}

impl<G, N, E> NodeRef<G, N, E> for NodeIdx {
	fn lookup(self, _g: &Graph<G, N, E>) -> Option<Self> {
		Some(self)
	}
}
impl<G, N, E> NodeRef<G, N, E> for &NodeIdx {
	fn lookup(self, _g: &Graph<G, N, E>) -> Option<NodeIdx> {
		Some(*self)
	}
}
impl<G, N, E> NodeRef<G, N, E> for &str {
	fn lookup(self, g: &Graph<G, N, E>) -> Option<NodeIdx> {
		g.node_idx(self)
	}
}
impl<G, N, E> NodeRef<G, N, E> for &SmolStr {
	fn lookup(self, g: &Graph<G, N, E>) -> Option<NodeIdx> {
		g.node_idx(self)
	}
}
impl<G, N, E> NodeRef<G, N, E> for SmolStr {
	fn lookup(self, g: &Graph<G, N, E>) -> Option<NodeIdx> {
		g.node_idx(&self)
	}
}
impl<G, N, E> NodeRef<G, N, E> for String {
	fn lookup(self, g: &Graph<G, N, E>) -> Option<NodeIdx> {
		g.node_idx(&self)
	}
}
impl<G, N, E> NodeRef<G, N, E> for &String {
	fn lookup(self, g: &Graph<G, N, E>) -> Option<NodeIdx> {
		g.node_idx(self)
	}
}

type NodeLabelFactory<N> = Box<dyn Fn(NodeIdx) -> N>;
type EdgeLabelFactory<E> = Box<dyn Fn(Edge) -> E>;

/// One live node. Adjacency is stored as edge-slot indices in insertion
/// order; degrees in dagre graphs are small (~4 before normalize, ~1 after),
/// so linear scans beat any map here.
#[derive(Debug, Clone)]
struct NodeSlot<N> {
	label: N,
	out: Vec<EdgeIdx>,
	inc: Vec<EdgeIdx>,
	/// Compound graphs only. `None` means the node sits at the root.
	parent: Option<NodeIdx>,
	/// Compound graphs only, insertion-ordered.
	children: Vec<NodeIdx>,
}

impl<N> NodeSlot<N> {
	const fn new(label: N) -> Self {
		Self {
			label,
			out: Vec::new(),
			inc: Vec::new(),
			parent: None,
			children: Vec::new(),
		}
	}
}

#[derive(Debug, Clone)]
struct EdgeSlot<E> {
	edge: Edge,
	label: E,
}

/// The graph itself. `G`, `N`, `E` are the label types.
///
/// Removed nodes and edges leave tombstones (`None` slots) behind and their
/// indices are never reused: `normalize::undo` deletes tens of thousands of
/// dummy nodes and adds none back, so a free list would buy nothing while
/// opening the door to a stale [`NodeIdx`] silently aliasing a new node.
pub struct Graph<G, N, E> {
	is_directed: bool,
	is_multigraph: bool,
	is_compound: bool,

	label: Option<G>,

	// Default label factory used by `set_node_default(v)` (without label).
	default_node_label: Option<NodeLabelFactory<N>>,
	default_edge_label: Option<EdgeLabelFactory<E>>,

	/// Node slots. Index is the [`NodeIdx`]; `None` is a tombstone.
	nodes: Vec<Option<NodeSlot<N>>>,
	live_nodes: usize,

	/// Edge slots. Index is the [`EdgeIdx`]; `None` is a tombstone.
	edges: Vec<Option<EdgeSlot<E>>>,
	live_edges: usize,
	/// Edge identity -> slot.
	edge_lookup: FxHashMap<Edge, EdgeIdx>,

	next_edge_name: u32,

	/// Optional human-readable node names. Empty unless a caller uses the
	/// `*_named` helpers; the layout pipeline never reads it.
	names: Vec<Option<SmolStr>>,
	by_name: FxHashMap<SmolStr, NodeIdx>,
}

impl<G, N, E> Graph<G, N, E> {
	pub fn new() -> Self {
		Self::with_opts(GraphOpts::directed())
	}

	pub fn with_opts(opts: GraphOpts) -> Self {
		Self {
			is_directed: opts.directed,
			is_multigraph: opts.multigraph,
			is_compound: opts.compound,
			label: None,
			default_node_label: None,
			default_edge_label: None,
			nodes: Vec::new(),
			live_nodes: 0,
			edges: Vec::new(),
			live_edges: 0,
			edge_lookup: FxHashMap::default(),
			next_edge_name: 0,
			names: Vec::new(),
			by_name: FxHashMap::default(),
		}
	}

	/// Pre-size the node table. Derived graphs that mirror a parent's index
	/// space use this to avoid repeated growth.
	pub fn reserve_nodes(&mut self, n: usize) {
		self.nodes.reserve(n);
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

	/// Upper bound on any live [`NodeIdx`]. Side tables indexed by node use
	/// this as their length.
	pub const fn node_bound(&self) -> usize {
		self.nodes.len()
	}
	/// Upper bound on any live [`EdgeIdx`].
	pub const fn edge_bound(&self) -> usize {
		self.edges.len()
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

	pub fn set_default_node_label<F: Fn(NodeIdx) -> N + 'static>(
		&mut self,
		f: F,
	) -> &mut Self {
		self.default_node_label = Some(Box::new(f));
		self
	}
	pub fn set_default_edge_label<F: Fn(Edge) -> E + 'static>(
		&mut self,
		f: F,
	) -> &mut Self {
		self.default_edge_label = Some(Box::new(f));
		self
	}

	// ---- nodes ----------------------------------------------------------

	pub const fn node_count(&self) -> usize {
		self.live_nodes
	}

	/// Snapshot of the live node ids, in index (= insertion) order. Cheap
	/// now that ids are `Copy` u32s; use it when the loop body mutates the
	/// graph, and `nodes_iter` otherwise.
	pub fn nodes(&self) -> Vec<NodeIdx> {
		self.nodes_iter().collect()
	}

	pub fn nodes_iter(&self) -> impl Iterator<Item = NodeIdx> + '_ {
		self.nodes
			.iter()
			.enumerate()
			.filter_map(|(i, slot)| {
				slot.as_ref()
					.map(|_| NodeIdx(u32::try_from(i).unwrap_or(u32::MAX)))
			})
	}

	pub fn has_node(&self, v: impl NodeRef<G, N, E>) -> bool {
		v.lookup(self)
			.is_some_and(|v| self.slot(v).is_some())
	}

	fn slot(&self, v: NodeIdx) -> Option<&NodeSlot<N>> {
		self.nodes.get(v.index())?.as_ref()
	}
	fn slot_mut(&mut self, v: NodeIdx) -> Option<&mut NodeSlot<N>> {
		self.nodes.get_mut(v.index())?.as_mut()
	}

	/// Append a node with a fresh index.
	pub fn add_node(&mut self, label: N) -> NodeIdx {
		let v =
			NodeIdx(u32::try_from(self.nodes.len()).expect("node overflow"));
		self.nodes
			.push(Some(NodeSlot::new(label)));
		self.live_nodes += 1;
		v
	}

	/// Create or replace the node at `v`, growing the slot table with
	/// tombstones as needed. Derived graphs (`util::simplify`,
	/// `order::build_layer_graph`, the block graph, the feasible tree) use
	/// this to share their parent's index space.
	pub fn set_node(
		&mut self,
		v: impl NodeKey<G, N, E>,
		label: N,
	) -> &mut Self {
		let v = v.resolve(self);
		if self.nodes.len() <= v.index() {
			self.nodes
				.resize_with(v.index() + 1, || None);
		}
		let slot = &mut self.nodes[v.index()];
		if let Some(existing) = slot {
			existing.label = label;
		} else {
			*slot = Some(NodeSlot::new(label));
			self.live_nodes += 1;
		}
		self
	}

	/// Like `set_node(v)` with no label: uses the default label factory if one
	/// was registered, else `N::default()`. Existing nodes are left alone.
	pub fn set_node_default(&mut self, v: impl NodeKey<G, N, E>) -> &mut Self
	where
		N: Default,
	{
		let v = v.resolve(self);
		if self.has_node(v) {
			return self;
		}
		let label = match &self.default_node_label {
			Some(f) => f(v),
			None => N::default(),
		};
		self.set_node(v, label)
	}

	pub fn node(&self, v: impl NodeRef<G, N, E>) -> Option<&N> {
		self.slot(v.lookup(self)?)
			.map(|s| &s.label)
	}
	pub fn node_mut(&mut self, v: impl NodeRef<G, N, E>) -> Option<&mut N> {
		let v = v.lookup(self)?;
		self.slot_mut(v).map(|s| &mut s.label)
	}

	pub fn remove_node(&mut self, v: impl NodeRef<G, N, E>) {
		let Some(v) = v.lookup(self) else {
			return;
		};
		if !self.has_node(v) {
			return;
		}
		if self.is_compound {
			// Children become root-level; detach from our own parent.
			let children = mem::take(&mut self.slot_mut(v).unwrap().children);
			for c in children {
				if let Some(cs) = self.slot_mut(c) {
					cs.parent = None;
				}
			}
			if let Some(p) = self.slot(v).and_then(|s| s.parent)
				&& let Some(ps) = self.slot_mut(p)
			{
				ps.children.retain(|&c| c != v);
			}
		}

		// Remove incident edges. Collect first: `remove_edge_obj` mutates
		// the adjacency lists we would otherwise be iterating.
		let incident: Vec<Edge> = {
			let slot = self.slot(v).unwrap();
			slot.inc
				.iter()
				.chain(slot.out.iter())
				.filter_map(|&e| self.edges[e.index()].as_ref())
				.map(|s| s.edge)
				.collect()
		};
		for e in incident {
			self.remove_edge_obj(&e);
		}

		self.nodes[v.index()] = None;
		self.live_nodes -= 1;
		if let Some(slot) = self.names.get_mut(v.index())
			&& let Some(name) = slot.take()
		{
			self.by_name.remove(&name);
		}
	}

	// ---- names (side table, not used by the layout pipeline) ------------

	/// Create a node carrying `name`, or replace the label of the node that
	/// already has it.
	pub fn set_node_named(
		&mut self,
		name: impl Into<SmolStr>,
		label: N,
	) -> NodeIdx {
		let name = name.into();
		if let Some(&v) = self.by_name.get(&name) {
			self.set_node(v, label);
			return v;
		}
		let v = self.add_node(label);
		self.bind_name(v, name);
		v
	}

	/// Look up (or create, with the default label) the node called `name`.
	pub fn node_named_or_insert(&mut self, name: &str) -> NodeIdx
	where
		N: Default,
	{
		if let Some(&v) = self.by_name.get(name) {
			return v;
		}
		let next =
			NodeIdx(u32::try_from(self.nodes.len()).expect("node overflow"));
		let label = match &self.default_node_label {
			Some(f) => f(next),
			None => N::default(),
		};
		let v = self.add_node(label);
		self.bind_name(v, SmolStr::from(name));
		v
	}

	fn bind_name(&mut self, v: NodeIdx, name: SmolStr) {
		if self.names.len() <= v.index() {
			self.names
				.resize_with(v.index() + 1, || None);
		}
		self.names[v.index()] = Some(name.clone());
		self.by_name.insert(name, v);
	}

	/// Index of the node called `name`, if one was ever registered.
	pub fn node_idx(&self, name: &str) -> Option<NodeIdx> {
		self.by_name.get(name).copied()
	}

	/// Name of `v`, if it has one.
	pub fn name(&self, v: NodeIdx) -> Option<&str> {
		self.names.get(v.index())?.as_deref()
	}

	/// `name(v)` with a `#idx` fallback, for diagnostics.
	pub fn name_or_idx(&self, v: NodeIdx) -> String {
		self.name(v)
			.map_or_else(|| v.to_string(), ToString::to_string)
	}

	pub fn names_of(&self, vs: &[NodeIdx]) -> Vec<String> {
		vs.iter()
			.map(|&v| self.name_or_idx(v))
			.collect()
	}

	/// Convenience: like graphlib's `setPath`, creates a chain of named nodes
	/// and the edges between them. Each edge gets the default edge label.
	pub fn set_path(&mut self, path: &[&str])
	where
		N: Default,
		E: Default,
	{
		let Some(first) = path.first() else {
			return;
		};
		let mut prev = self.node_named_or_insert(first);
		for name in &path[1..] {
			let cur = self.node_named_or_insert(name);
			self.set_edge_full(prev, cur, None, None);
			prev = cur;
		}
	}

	// ---- compound -------------------------------------------------------

	pub fn set_parent(
		&mut self,
		v: impl NodeKey<G, N, E>,
		parent: Option<impl NodeKey<G, N, E>>,
	) where
		N: Default,
	{
		let v = v.resolve(self);
		let parent = parent.map(|p| p.resolve(self));
		assert!(
			self.is_compound,
			"Cannot set parent in a non-compound graph"
		);
		self.set_node_default(v);
		if let Some(p) = parent {
			self.set_node_default(p);
			// Disallow cycles.
			let mut ancestor = Some(p);
			while let Some(a) = ancestor {
				assert!(
					a != v,
					"Setting {p} as parent of {v} would create a cycle"
				);
				ancestor = self.slot(a).and_then(|s| s.parent);
			}
		}
		// Detach from previous parent.
		if let Some(prev) = self.slot(v).and_then(|s| s.parent)
			&& let Some(ps) = self.slot_mut(prev)
		{
			ps.children.retain(|&c| c != v);
		}
		if let Some(s) = self.slot_mut(v) {
			s.parent = parent;
		}
		if let Some(p) = parent
			&& let Some(ps) = self.slot_mut(p)
			&& !ps.children.contains(&v)
		{
			ps.children.push(v);
		}
	}

	/// Detach `v` from its parent, making it root-level. Separate from
	/// `set_parent` because a bare `None` there cannot infer a key type.
	pub fn unset_parent(&mut self, v: impl NodeKey<G, N, E>)
	where
		N: Default,
	{
		self.set_parent(v, None::<NodeIdx>);
	}

	pub fn parent(&self, v: impl NodeRef<G, N, E>) -> Option<NodeIdx> {
		if !self.is_compound {
			return None;
		}
		self.slot(v.lookup(self)?)?.parent
	}

	/// The root-level nodes. Counterpart to `children` for the same reason
	/// `unset_parent` exists: a bare `None` cannot infer a key type.
	pub fn root_children(&self) -> Vec<NodeIdx> {
		self.children(None::<NodeIdx>)
	}

	/// Children of `v`, or the root-level nodes if `v` is `None`.
	pub fn children(&self, v: Option<impl NodeRef<G, N, E>>) -> Vec<NodeIdx> {
		let v = v.map(|v| v.lookup(self));
		if !self.is_compound {
			return match v {
				None => self.nodes(),
				Some(_) => Vec::new(),
			};
		}
		match v.flatten() {
			Some(v) => self
				.slot(v)
				.map(|s| s.children.clone())
				.unwrap_or_default(),
			None => self
				.nodes_iter()
				.filter(|&n| {
					self.slot(n)
						.is_some_and(|s| s.parent.is_none())
				})
				.collect(),
		}
	}

	// ---- edges ----------------------------------------------------------

	pub const fn edge_count(&self) -> usize {
		self.live_edges
	}

	/// Mint a fresh edge name. The value is opaque; it exists only to keep
	/// parallel edges distinct.
	pub const fn fresh_edge_name(&mut self) -> EdgeName {
		let n = EdgeName(self.next_edge_name);
		self.next_edge_name += 1;
		n
	}

	pub fn edges(&self) -> Vec<Edge> {
		self.edges_iter().collect()
	}

	pub fn edges_iter(&self) -> impl Iterator<Item = Edge> + '_ {
		self.edges
			.iter()
			.filter_map(|slot| slot.as_ref().map(|s| s.edge))
	}

	/// Canonical identity for `(v, w, name)`. Undirected graphs order the
	/// endpoints so both directions map to the same edge.
	const fn canonical(&self, e: Edge) -> Edge {
		if !self.is_directed && e.v.0 > e.w.0 {
			Edge {
				v: e.w,
				w: e.v,
				name: e.name,
			}
		} else {
			e
		}
	}

	pub fn has_edge(
		&self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
	) -> bool {
		self.has_edge_named(v, w, None)
	}
	pub fn has_edge_named(
		&self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
		name: Option<EdgeName>,
	) -> bool {
		let (Some(v), Some(w)) = (v.lookup(self), w.lookup(self)) else {
			return false;
		};
		self.has_edge_obj(&Edge { v, w, name })
	}
	pub fn has_edge_obj(&self, e: &Edge) -> bool {
		self.edge_lookup
			.contains_key(&self.canonical(*e))
	}

	pub fn set_edge(
		&mut self,
		v: impl NodeKey<G, N, E>,
		w: impl NodeKey<G, N, E>,
		label: E,
	) where
		N: Default,
		E: Default,
	{
		self.set_edge_named(v, w, label, None);
	}

	pub fn set_edge_named(
		&mut self,
		v: impl NodeKey<G, N, E>,
		w: impl NodeKey<G, N, E>,
		label: E,
		name: Option<EdgeName>,
	) where
		N: Default,
		E: Default,
	{
		let v = v.resolve(self);
		let w = w.resolve(self);
		self.set_edge_full(v, w, name, Some(label));
	}

	/// Equivalent to `set_edge(v, w)` with no label - applies default factory.
	pub fn set_edge_default(
		&mut self,
		v: impl NodeKey<G, N, E>,
		w: impl NodeKey<G, N, E>,
	) where
		N: Default,
		E: Default,
	{
		let v = v.resolve(self);
		let w = w.resolve(self);
		self.set_edge_full(v, w, None, None);
	}

	pub fn set_edge_obj(&mut self, e: &Edge, label: E)
	where
		N: Default,
		E: Default,
	{
		self.set_edge_full(e.v, e.w, e.name, Some(label));
	}
	pub fn set_edge_obj_default(&mut self, e: &Edge)
	where
		N: Default,
		E: Default,
	{
		self.set_edge_full(e.v, e.w, e.name, None);
	}

	fn set_edge_full(
		&mut self,
		v: NodeIdx,
		w: NodeIdx,
		name: Option<EdgeName>,
		label: Option<E>,
	) where
		N: Default,
		E: Default,
	{
		assert!(
			name.is_none() || self.is_multigraph,
			"Cannot set a named edge when isMultigraph = false"
		);
		let edge = self.canonical(Edge { v, w, name });

		if let Some(&idx) = self.edge_lookup.get(&edge) {
			let label = match label {
				Some(l) => l,
				None => match &self.default_edge_label {
					Some(f) => f(edge),
					None => E::default(),
				},
			};
			if let Some(slot) = self.edges[idx.index()].as_mut() {
				slot.label = label;
			}
			return;
		}

		self.set_node_default(edge.v);
		self.set_node_default(edge.w);

		let label = match label {
			Some(l) => l,
			None => match &self.default_edge_label {
				Some(f) => f(edge),
				None => E::default(),
			},
		};

		let idx =
			EdgeIdx(u32::try_from(self.edges.len()).expect("edge overflow"));
		self.edges
			.push(Some(EdgeSlot { edge, label }));
		self.live_edges += 1;
		self.edge_lookup.insert(edge, idx);
		self.nodes[edge.v.index()]
			.as_mut()
			.expect("edge tail exists")
			.out
			.push(idx);
		self.nodes[edge.w.index()]
			.as_mut()
			.expect("edge head exists")
			.inc
			.push(idx);
	}

	pub fn edge(
		&self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
	) -> Option<&E> {
		self.edge_full(v, w, None)
	}
	pub fn edge_mut(
		&mut self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
	) -> Option<&mut E> {
		self.edge_full_mut(v, w, None)
	}
	pub fn edge_full(
		&self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
		name: Option<EdgeName>,
	) -> Option<&E> {
		let (v, w) = (v.lookup(self)?, w.lookup(self)?);
		self.edge_obj(&Edge { v, w, name })
	}
	pub fn edge_full_mut(
		&mut self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
		name: Option<EdgeName>,
	) -> Option<&mut E> {
		let (v, w) = (v.lookup(self)?, w.lookup(self)?);
		self.edge_obj_mut(&Edge { v, w, name })
	}
	pub fn edge_obj(&self, e: &Edge) -> Option<&E> {
		let idx = *self
			.edge_lookup
			.get(&self.canonical(*e))?;
		self.edges[idx.index()]
			.as_ref()
			.map(|s| &s.label)
	}
	pub fn edge_obj_mut(&mut self, e: &Edge) -> Option<&mut E> {
		let idx = *self
			.edge_lookup
			.get(&self.canonical(*e))?;
		self.edges[idx.index()]
			.as_mut()
			.map(|s| &mut s.label)
	}

	pub fn remove_edge(
		&mut self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
	) {
		self.remove_edge_named(v, w, None);
	}
	pub fn remove_edge_named(
		&mut self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
		name: Option<EdgeName>,
	) {
		let (Some(v), Some(w)) = (v.lookup(self), w.lookup(self)) else {
			return;
		};
		self.remove_edge_obj(&Edge { v, w, name });
	}
	pub fn remove_edge_obj(&mut self, e: &Edge) {
		let edge = self.canonical(*e);
		let Some(idx) = self.edge_lookup.remove(&edge) else {
			return;
		};
		self.edges[idx.index()] = None;
		self.live_edges -= 1;
		if let Some(s) = self.slot_mut(edge.v) {
			s.out.retain(|&x| x != idx);
		}
		if let Some(s) = self.slot_mut(edge.w) {
			s.inc.retain(|&x| x != idx);
		}
	}

	// ---- edge slots -----------------------------------------------------
	//
	// Hot loops address edges by slot so they never hash an `Edge`: the
	// adjacency lists are borrowed as-is and the label is a direct index.

	/// Outgoing edge slots of `v`, in insertion order.
	pub fn out_edge_idxs(&self, v: NodeIdx) -> &[EdgeIdx] {
		self.slot(v).map_or(&[], |s| &s.out)
	}
	/// Incoming edge slots of `v`, in insertion order.
	pub fn in_edge_idxs(&self, v: NodeIdx) -> &[EdgeIdx] {
		self.slot(v).map_or(&[], |s| &s.inc)
	}
	/// Endpoints of an edge slot.
	pub fn edge_ends(&self, e: EdgeIdx) -> Option<Edge> {
		Some(
			self.edges
				.get(e.index())?
				.as_ref()?
				.edge,
		)
	}
	/// Label of an edge slot.
	pub fn edge_label(&self, e: EdgeIdx) -> Option<&E> {
		Some(
			&self
				.edges
				.get(e.index())?
				.as_ref()?
				.label,
		)
	}
	pub fn edge_label_mut(&mut self, e: EdgeIdx) -> Option<&mut E> {
		Some(
			&mut self
				.edges
				.get_mut(e.index())?
				.as_mut()?
				.label,
		)
	}
	/// Endpoints and label together, for loops that need both.
	pub fn edge_entry(&self, e: EdgeIdx) -> Option<(Edge, &E)> {
		let slot = self.edges.get(e.index())?.as_ref()?;
		Some((slot.edge, &slot.label))
	}

	// ---- neighborhood ---------------------------------------------------

	fn edges_of(&self, list: &[EdgeIdx]) -> Vec<Edge> {
		list.iter()
			.filter_map(|&i| self.edges[i.index()].as_ref())
			.map(|s| s.edge)
			.collect()
	}

	pub fn in_edges(&self, v: impl NodeRef<G, N, E>) -> Option<Vec<Edge>> {
		let slot = self.slot(v.lookup(self)?)?;
		Some(self.edges_of(&slot.inc))
	}
	pub fn in_edges_iter(
		&self,
		v: impl NodeRef<G, N, E>,
	) -> Option<impl Iterator<Item = Edge> + '_> {
		let slot = self.slot(v.lookup(self)?)?;
		Some(
			slot.inc
				.iter()
				.filter_map(|&i| self.edges[i.index()].as_ref())
				.map(|s| s.edge),
		)
	}
	pub fn in_edges_from(
		&self,
		v: impl NodeRef<G, N, E>,
		u: impl NodeRef<G, N, E>,
	) -> Option<Vec<Edge>> {
		let u = u.lookup(self)?;
		Some(
			self.in_edges_iter(v)?
				.filter(|e| e.v == u)
				.collect(),
		)
	}
	pub fn out_edges(&self, v: impl NodeRef<G, N, E>) -> Option<Vec<Edge>> {
		let slot = self.slot(v.lookup(self)?)?;
		Some(self.edges_of(&slot.out))
	}
	pub fn out_edges_iter(
		&self,
		v: impl NodeRef<G, N, E>,
	) -> Option<impl Iterator<Item = Edge> + '_> {
		let slot = self.slot(v.lookup(self)?)?;
		Some(
			slot.out
				.iter()
				.filter_map(|&i| self.edges[i.index()].as_ref())
				.map(|s| s.edge),
		)
	}
	pub fn out_edges_to(
		&self,
		v: impl NodeRef<G, N, E>,
		w: impl NodeRef<G, N, E>,
	) -> Option<Vec<Edge>> {
		let w = w.lookup(self)?;
		Some(
			self.out_edges_iter(v)?
				.filter(|e| e.w == w)
				.collect(),
		)
	}
	pub fn node_edges(&self, v: impl NodeRef<G, N, E>) -> Option<Vec<Edge>> {
		let v = v.lookup(self)?;
		let mut out = self.out_edges(v)?;
		out.extend(self.in_edges(v)?);
		Some(out)
	}

	/// Distinct tails of the incoming edges, appended to `out`. Scratch-buffer
	/// form for hot loops that would otherwise allocate a `Vec` per node.
	pub fn predecessors_into(&self, v: NodeIdx, out: &mut Vec<NodeIdx>) {
		out.clear();
		for &ei in self.in_edge_idxs(v) {
			if let Some(slot) = self.edges[ei.index()].as_ref()
				&& !out.contains(&slot.edge.v)
			{
				out.push(slot.edge.v);
			}
		}
	}

	/// Distinct heads of the outgoing edges, appended to `out`.
	pub fn successors_into(&self, v: NodeIdx, out: &mut Vec<NodeIdx>) {
		out.clear();
		for &ei in self.out_edge_idxs(v) {
			if let Some(slot) = self.edges[ei.index()].as_ref()
				&& !out.contains(&slot.edge.w)
			{
				out.push(slot.edge.w);
			}
		}
	}

	/// Distinct tails of the incoming edges, in edge-insertion order.
	pub fn predecessors(
		&self,
		v: impl NodeRef<G, N, E>,
	) -> Option<Vec<NodeIdx>> {
		let mut out: Vec<NodeIdx> = Vec::new();
		for e in self.in_edges_iter(v)? {
			if !out.contains(&e.v) {
				out.push(e.v);
			}
		}
		Some(out)
	}
	/// Distinct heads of the outgoing edges, in edge-insertion order.
	pub fn successors(&self, v: impl NodeRef<G, N, E>) -> Option<Vec<NodeIdx>> {
		let mut out: Vec<NodeIdx> = Vec::new();
		for e in self.out_edges_iter(v)? {
			if !out.contains(&e.w) {
				out.push(e.w);
			}
		}
		Some(out)
	}
	pub fn neighbors(&self, v: impl NodeRef<G, N, E>) -> Option<Vec<NodeIdx>> {
		let v = v.lookup(self)?;
		let mut s = self.predecessors(v)?;
		for n in self.successors(v)? {
			if !s.contains(&n) {
				s.push(n);
			}
		}
		Some(s)
	}
	pub fn sources(&self) -> Vec<NodeIdx> {
		self.nodes_iter()
			.filter(|&v| {
				self.slot(v)
					.is_some_and(|s| s.inc.is_empty())
			})
			.collect()
	}
	pub fn sinks(&self) -> Vec<NodeIdx> {
		self.nodes_iter()
			.filter(|&v| {
				self.slot(v)
					.is_some_and(|s| s.out.is_empty())
			})
			.collect()
	}
}

impl<G, N, E> Default for Graph<G, N, E> {
	fn default() -> Self {
		Self::new()
	}
}

impl<G: std::fmt::Debug, N: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug
	for Graph<G, N, E>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Graph")
			.field("is_directed", &self.is_directed)
			.field("is_multigraph", &self.is_multigraph)
			.field("is_compound", &self.is_compound)
			.field("label", &self.label)
			.field("node_count", &self.live_nodes)
			.field("edge_count", &self.live_edges)
			.finish_non_exhaustive()
	}
}

// ---- serde --------------------------------------------------------------

/// Wire format. Deliberately *not* a mirror of the internal storage: the
/// adjacency lists, the edge lookup and the name map are all rebuilt on
/// load, so only nodes and edges are transmitted.
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
#[serde(bound(
	serialize = "G: Serialize, N: Serialize, E: Serialize",
	deserialize = "G: Deserialize<'de>, N: Deserialize<'de>, E: Deserialize<'de>"
))]
struct SerGraph<G, N, E> {
	directed: bool,
	multigraph: bool,
	compound: bool,
	label: Option<G>,
	/// Position in this array *is* the `NodeIdx`. `None` is a tombstone.
	nodes: Vec<Option<SerNode<N>>>,
	edges: Vec<SerEdge<E>>,
	#[serde(default)]
	next_edge_name: u32,
}

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct SerNode<N> {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	name: Option<SmolStr>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	parent: Option<NodeIdx>,
	label: N,
}

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct SerEdge<E> {
	v: NodeIdx,
	w: NodeIdx,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	name: Option<EdgeName>,
	label: E,
}

#[cfg(feature = "serde")]
impl<G, N, E> Serialize for Graph<G, N, E>
where
	G: Serialize + Clone,
	N: Serialize + Clone,
	E: Serialize + Clone,
{
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		let nodes = self
			.nodes
			.iter()
			.enumerate()
			.map(|(i, slot)| {
				slot.as_ref().map(|s| SerNode {
					name: self.names.get(i).cloned().flatten(),
					parent: s.parent,
					label: s.label.clone(),
				})
			})
			.collect();
		let edges = self
			.edges
			.iter()
			.filter_map(|slot| {
				slot.as_ref().map(|s| SerEdge {
					v: s.edge.v,
					w: s.edge.w,
					name: s.edge.name,
					label: s.label.clone(),
				})
			})
			.collect();
		SerGraph {
			directed: self.is_directed,
			multigraph: self.is_multigraph,
			compound: self.is_compound,
			label: self.label.clone(),
			nodes,
			edges,
			next_edge_name: self.next_edge_name,
		}
		.serialize(serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de, G, N, E> Deserialize<'de> for Graph<G, N, E>
where
	G: Deserialize<'de>,
	N: Deserialize<'de> + Default,
	E: Deserialize<'de> + Default,
{
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		let raw: SerGraph<G, N, E> = SerGraph::deserialize(deserializer)?;
		let mut g = Self::with_opts(GraphOpts {
			directed: raw.directed,
			multigraph: raw.multigraph,
			compound: raw.compound,
		});
		g.label = raw.label;
		g.next_edge_name = raw.next_edge_name;
		g.nodes.reserve(raw.nodes.len());
		g.names.reserve(raw.nodes.len());
		let mut parents: Vec<(NodeIdx, NodeIdx)> = Vec::new();
		for (i, node) in raw.nodes.into_iter().enumerate() {
			let v = NodeIdx(u32::try_from(i).expect("node overflow"));
			match node {
				Some(n) => {
					g.set_node(v, n.label);
					if let Some(name) = n.name {
						g.bind_name(v, name);
					}
					if let Some(p) = n.parent {
						parents.push((v, p));
					}
				}
				None => g.nodes.push(None),
			}
		}
		for (v, p) in parents {
			if let Some(s) = g.slot_mut(v) {
				s.parent = Some(p);
			}
			if let Some(ps) = g.slot_mut(p) {
				ps.children.push(v);
			}
		}
		for e in raw.edges {
			g.set_edge_full(e.v, e.w, e.name, Some(e.label));
		}
		Ok(g)
	}
}

// ---- graph algorithms used by network-simplex --------------------------

pub mod alg {
	use super::{Graph, NodeIdx};

	/// Postorder DFS traversal - used by network-simplex.
	pub fn postorder<G, N, E>(
		g: &Graph<G, N, E>,
		starts: &[NodeIdx],
	) -> Vec<NodeIdx> {
		let mut visited = vec![false; g.node_bound()];
		let mut result: Vec<NodeIdx> = Vec::new();
		for s in starts {
			dfs(g, *s, &mut visited, &mut result, true);
		}
		result
	}

	pub fn preorder<G, N, E>(
		g: &Graph<G, N, E>,
		starts: &[NodeIdx],
	) -> Vec<NodeIdx> {
		let mut visited = vec![false; g.node_bound()];
		let mut result: Vec<NodeIdx> = Vec::new();
		for s in starts {
			dfs(g, *s, &mut visited, &mut result, false);
		}
		result
	}

	fn dfs<G, N, E>(
		g: &Graph<G, N, E>,
		v: NodeIdx,
		visited: &mut [bool],
		result: &mut Vec<NodeIdx>,
		postorder: bool,
	) {
		if visited[v.index()] {
			return;
		}
		visited[v.index()] = true;
		if !postorder {
			result.push(v);
		}
		let next = if g.is_directed() {
			g.successors(v).unwrap_or_default()
		} else {
			g.neighbors(v).unwrap_or_default()
		};
		for w in next {
			dfs(g, w, visited, result, postorder);
		}
		if postorder {
			result.push(v);
		}
	}

	/// Tarjan's strongly-connected components.
	pub fn tarjan<G, N, E>(g: &Graph<G, N, E>) -> Vec<Vec<NodeIdx>> {
		struct State {
			index: usize,
			stack: Vec<NodeIdx>,
			on_stack: Vec<bool>,
			indices: Vec<Option<usize>>,
			lowlinks: Vec<usize>,
			results: Vec<Vec<NodeIdx>>,
		}

		fn strong_connect<G, N, E>(
			s: &mut State,
			g: &Graph<G, N, E>,
			v: NodeIdx,
		) {
			s.indices[v.index()] = Some(s.index);
			s.lowlinks[v.index()] = s.index;
			s.index += 1;
			s.stack.push(v);
			s.on_stack[v.index()] = true;

			for w in g.successors(v).unwrap_or_default() {
				if s.indices[w.index()].is_none() {
					strong_connect(s, g, w);
					s.lowlinks[v.index()] =
						s.lowlinks[v.index()].min(s.lowlinks[w.index()]);
				} else if s.on_stack[w.index()] {
					s.lowlinks[v.index()] = s.lowlinks[v.index()]
						.min(s.indices[w.index()].unwrap_or(usize::MAX));
				}
			}

			if Some(s.lowlinks[v.index()]) == s.indices[v.index()] {
				let mut comp: Vec<NodeIdx> = Vec::new();
				while let Some(w) = s.stack.pop() {
					s.on_stack[w.index()] = false;
					comp.push(w);
					if w == v {
						break;
					}
				}
				s.results.push(comp);
			}
		}

		let bound = g.node_bound();
		let mut state = State {
			index: 0,
			stack: Vec::new(),
			on_stack: vec![false; bound],
			indices: vec![None; bound],
			lowlinks: vec![0; bound],
			results: Vec::new(),
		};
		for v in g.nodes() {
			if state.indices[v.index()].is_none() {
				strong_connect(&mut state, g, v);
			}
		}
		state.results
	}

	/// Strongly-connected components with more than one node, plus
	/// single-node components that carry a self-loop.
	pub fn find_cycles<G, N, E>(g: &Graph<G, N, E>) -> Vec<Vec<NodeIdx>> {
		tarjan(g)
			.into_iter()
			.filter(|comp| {
				comp.len() > 1
					|| (comp.len() == 1 && g.has_edge(comp[0], comp[0]))
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::{Edge, Graph, GraphOpts, NodeIdx};

	#[test]
	fn add_and_read_nodes() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		let a = g.add_node(1);
		let b = g.add_node(2);
		assert_eq!(g.node_count(), 2);
		assert_eq!(g.node(a), Some(&1));
		assert_eq!(g.node(b), Some(&2));
		assert_eq!(g.nodes(), vec![a, b]);
	}

	#[test]
	fn remove_node_tombstones_and_keeps_order() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		let a = g.add_node(1);
		let b = g.add_node(2);
		let c = g.add_node(3);
		g.remove_node(b);
		assert_eq!(g.node_count(), 2);
		assert_eq!(g.nodes(), vec![a, c]);
		assert!(!g.has_node(b));
	}

	#[test]
	fn edges_and_adjacency() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		let a = g.add_node(0);
		let b = g.add_node(0);
		let c = g.add_node(0);
		g.set_edge(a, b, 1);
		g.set_edge(b, c, 2);
		assert_eq!(g.edge_count(), 2);
		assert_eq!(g.edge(a, b), Some(&1));
		assert_eq!(g.successors(a), Some(vec![b]));
		assert_eq!(g.predecessors(c), Some(vec![b]));
		assert_eq!(g.sources(), vec![a]);
		assert_eq!(g.sinks(), vec![c]);
		g.remove_edge(a, b);
		assert_eq!(g.edge_count(), 1);
		assert_eq!(g.successors(a), Some(vec![]));
	}

	#[test]
	fn named_edges_need_multigraph() {
		let mut g: Graph<(), i32, i32> =
			Graph::with_opts(GraphOpts::directed().multigraph());
		let a = g.add_node(0);
		let b = g.add_node(0);
		let n1 = g.fresh_edge_name();
		let n2 = g.fresh_edge_name();
		g.set_edge_named(a, b, 1, Some(n1));
		g.set_edge_named(a, b, 2, Some(n2));
		assert_eq!(g.edge_count(), 2);
		assert_eq!(g.edge_full(a, b, Some(n1)), Some(&1));
		assert_eq!(g.edge_full(a, b, Some(n2)), Some(&2));
	}

	#[test]
	fn undirected_edges_are_canonical() {
		let mut g: Graph<(), i32, i32> =
			Graph::with_opts(GraphOpts::undirected());
		let a = g.add_node(0);
		let b = g.add_node(0);
		g.set_edge(b, a, 7);
		assert_eq!(g.edge(a, b), Some(&7));
		assert!(g.has_edge(b, a));
		assert_eq!(g.edge_count(), 1);
	}

	#[test]
	fn names_are_optional_side_table() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		let a = g.set_node_named("a", 1);
		assert_eq!(g.node_idx("a"), Some(a));
		assert_eq!(g.name(a), Some("a"));
		let plain = g.add_node(2);
		assert_eq!(g.name(plain), None);
		assert_eq!(g.name_or_idx(plain), "#1");
	}

	#[test]
	fn set_path_builds_named_chain() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		g.set_path(&["a", "b", "c"]);
		let a = g.node_idx("a").unwrap();
		let b = g.node_idx("b").unwrap();
		let c = g.node_idx("c").unwrap();
		assert!(g.has_edge(a, b));
		assert!(g.has_edge(b, c));
		assert_eq!(g.node_count(), 3);
	}

	#[test]
	fn compound_parent_children() {
		let mut g: Graph<(), i32, i32> =
			Graph::with_opts(GraphOpts::directed().compound());
		let a = g.add_node(0);
		let sg = g.add_node(0);
		g.set_parent(a, Some(sg));
		assert_eq!(g.parent(a), Some(sg));
		assert_eq!(g.children(Some(sg)), vec![a]);
		assert_eq!(g.root_children(), vec![sg]);
		g.unset_parent(a);
		assert_eq!(g.parent(a), None);
		assert_eq!(g.children(Some(sg)), vec![]);
	}

	#[test]
	fn set_node_grows_with_holes() {
		let mut g: Graph<(), i32, i32> = Graph::new();
		g.set_node(NodeIdx(3), 9);
		assert_eq!(g.node_count(), 1);
		assert_eq!(g.node_bound(), 4);
		assert_eq!(g.nodes(), vec![NodeIdx(3)]);
	}

	#[cfg(feature = "serde")]
	#[test]
	fn serde_roundtrip() {
		let mut g: Graph<String, i32, i32> = Graph::new();
		g.set_graph("gl".to_string());
		let a = g.set_node_named("a", 1);
		let b = g.set_node_named("b", 2);
		let c = g.add_node(3);
		g.set_edge(a, b, 10);
		g.set_edge(b, c, 20);
		g.remove_node(c);

		let json = serde_json::to_string(&g).unwrap();
		let back: Graph<String, i32, i32> =
			serde_json::from_str(&json).unwrap();
		assert_eq!(back.graph(), Some(&"gl".to_string()));
		assert_eq!(back.node_count(), 2);
		assert_eq!(back.nodes(), vec![a, b]);
		assert_eq!(back.node_idx("a"), Some(a));
		assert_eq!(back.edge(a, b), Some(&10));
		assert_eq!(back.edge_count(), 1);
		assert_eq!(back.edges(), vec![Edge::new(a, b)]);
	}
}
