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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Identifies an edge by its endpoints and (for multigraphs) a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Edge {
	pub v: String,
	pub w: String,
	pub name: Option<String>,
}

impl Edge {
	pub fn new(v: impl Into<String>, w: impl Into<String>) -> Self {
		Edge {
			v: v.into(),
			w: w.into(),
			name: None,
		}
	}

	pub fn with_name(
		v: impl Into<String>,
		w: impl Into<String>,
		name: impl Into<String>,
	) -> Self {
		Edge {
			v: v.into(),
			w: w.into(),
			name: Some(name.into()),
		}
	}
}

/// Graph configuration. Default is a directed simple graph (matches graphlib).
#[derive(Debug, Clone, Copy)]
pub struct GraphOpts {
	pub directed: bool,
	pub multigraph: bool,
	pub compound: bool,
}

impl Default for GraphOpts {
	fn default() -> Self {
		GraphOpts {
			directed: true,
			multigraph: false,
			compound: false,
		}
	}
}

impl GraphOpts {
	pub fn directed() -> Self {
		GraphOpts::default()
	}
	pub fn undirected() -> Self {
		GraphOpts {
			directed: false,
			multigraph: false,
			compound: false,
		}
	}
	pub fn multigraph(mut self) -> Self {
		self.multigraph = true;
		self
	}
	pub fn compound(mut self) -> Self {
		self.compound = true;
		self
	}
}

/// The graph itself. `G`, `N`, `E` are the label types.
pub struct Graph<G, N, E> {
	is_directed: bool,
	is_multigraph: bool,
	is_compound: bool,

	label: Option<G>,

	// Default label factory used by `setNode(v)` (without label).
	default_node_label: Option<Box<dyn Fn(&str) -> N>>,
	default_edge_label: Option<Box<dyn Fn(&Edge) -> E>>,

	/// Insertion-ordered node ids.
	node_order: Vec<String>,
	node_index: HashMap<String, usize>,
	node_labels: HashMap<String, N>,

	/// Compound graph: parent of each node.
	parent: HashMap<String, String>,
	/// Children of each parent ("\x00" key = root children, matching JS impl).
	children: HashMap<String, BTreeSet<String>>,

	/// out[v] -> { edge_id -> Edge }
	out_edges: HashMap<String, BTreeMap<String, Edge>>,
	/// in[v] -> { edge_id -> Edge }
	in_edges: HashMap<String, BTreeMap<String, Edge>>,
	/// predecessor count: preds[v][u] = number of u->v edges
	preds: HashMap<String, HashMap<String, usize>>,
	sucs: HashMap<String, HashMap<String, usize>>,

	/// Insertion-ordered edge ids -> Edge.
	edge_order: Vec<String>,
	edge_index: HashMap<String, usize>,
	/// Edge label, keyed by edge_id.
	edge_labels: HashMap<String, E>,
	/// Edge object, keyed by edge_id.
	edge_objs: HashMap<String, Edge>,
}

const GRAPH_NODE: &str = "\x00";

impl<G, N, E> Graph<G, N, E> {
	pub fn new() -> Self {
		Self::with_opts(GraphOpts::directed())
	}

	pub fn with_opts(opts: GraphOpts) -> Self {
		let is_directed = opts.directed;
		let mut g = Graph {
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
				.insert(GRAPH_NODE.to_string(), BTreeSet::new());
		}
		g
	}

	pub fn is_directed(&self) -> bool {
		self.is_directed
	}
	pub fn is_multigraph(&self) -> bool {
		self.is_multigraph
	}
	pub fn is_compound(&self) -> bool {
		self.is_compound
	}

	// ---- graph label ----------------------------------------------------

	pub fn set_graph(&mut self, label: G) -> &mut Self {
		self.label = Some(label);
		self
	}
	pub fn graph(&self) -> Option<&G> {
		self.label.as_ref()
	}
	pub fn graph_mut(&mut self) -> Option<&mut G> {
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

	pub fn node_count(&self) -> usize {
		self.node_order.len()
	}
	pub fn nodes(&self) -> Vec<String> {
		self.node_order.clone()
	}
	pub fn has_node(&self, v: &str) -> bool {
		self.node_labels.contains_key(v)
	}

	pub fn set_node(&mut self, v: impl Into<String>, label: N) -> &mut Self {
		let v = v.into();
		if self.node_labels.contains_key(&v) {
			self.node_labels.insert(v, label);
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
				.insert(v.clone(), GRAPH_NODE.to_string());
			self.children
				.entry(GRAPH_NODE.to_string())
				.or_default()
				.insert(v.clone());
			self.children
				.insert(v.clone(), BTreeSet::new());
		}
		self
	}

	/// Like `setNode(v)` in JS with no label: uses default label factory if
	/// one was registered, else falls back to `N::default()`.
	pub fn set_node_default(&mut self, v: impl Into<String>) -> &mut Self
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
			let cs: Vec<String> = self
				.children
				.get(v)
				.map(|s| s.iter().cloned().collect())
				.unwrap_or_default();
			// Remove this v from its parent's child set.
			if let Some(p) = self.parent.remove(v) {
				if let Some(set) = self.children.get_mut(&p) {
					set.remove(v);
				}
			}
			for c in &cs {
				// children become root-level.
				self.parent
					.insert(c.clone(), GRAPH_NODE.to_string());
				self.children
					.entry(GRAPH_NODE.to_string())
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
		for e in in_es
			.into_iter()
			.chain(out_es.into_iter())
		{
			self.remove_edge_obj(&e);
		}

		self.node_labels.remove(v);
		self.in_edges.remove(v);
		self.out_edges.remove(v);
		self.preds.remove(v);
		self.sucs.remove(v);
		if let Some(&idx) = self.node_index.get(v) {
			self.node_order.remove(idx);
			// Reindex.
			self.node_index.clear();
			for (i, n) in self.node_order.iter().enumerate() {
				self.node_index.insert(n.clone(), i);
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
		let new_parent = parent.unwrap_or(GRAPH_NODE).to_string();
		// Disallow cycles.
		if parent.is_some() {
			let mut ancestor = Some(new_parent.clone());
			while let Some(a) = ancestor {
				if a == v {
					panic!(
						"Setting {} as parent of {} would create a cycle",
						new_parent, v
					);
				}
				ancestor = self.parent.get(&a).cloned();
				if ancestor.as_deref() == Some(GRAPH_NODE) {
					ancestor = None;
				}
			}
		}
		if parent.is_some() && !self.has_node(parent.unwrap()) {
			self.set_node_default(parent.unwrap().to_string());
		}
		// Detach from previous parent.
		if let Some(prev) = self.parent.get(v).cloned() {
			if let Some(set) = self.children.get_mut(&prev) {
				set.remove(v);
			}
		}
		self.parent
			.insert(v.to_string(), new_parent.clone());
		self.children
			.entry(new_parent)
			.or_default()
			.insert(v.to_string());
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
	pub fn children(&self, v: Option<&str>) -> Vec<String> {
		let key = v.unwrap_or(GRAPH_NODE);
		if self.is_compound {
			self.children
				.get(key)
				.map(|s| s.iter().cloned().collect())
				.unwrap_or_default()
		} else if v.is_none() {
			self.nodes()
		} else if self.has_node(v.unwrap()) {
			vec![]
		} else {
			vec![]
		}
	}

	// ---- edges ----------------------------------------------------------

	pub fn edge_count(&self) -> usize {
		self.edge_order.len()
	}

	pub fn edges(&self) -> Vec<Edge> {
		self.edge_order
			.iter()
			.map(|id| self.edge_objs.get(id).cloned().unwrap())
			.collect()
	}

	fn edge_id(&self, v: &str, w: &str, name: Option<&str>) -> String {
		if !self.is_directed && v > w {
			format!("{}\x01{}\x01{}", w, v, name.unwrap_or(""))
		} else {
			format!("{}\x01{}\x01{}", v, w, name.unwrap_or(""))
		}
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
		v: impl Into<String>,
		w: impl Into<String>,
		label: E,
	) where
		N: Default,
		E: Default,
	{
		self.set_edge_full(v, w, None, Some(label));
	}

	pub fn set_edge_named(
		&mut self,
		v: impl Into<String>,
		w: impl Into<String>,
		label: E,
		name: Option<String>,
	) where
		N: Default,
		E: Default,
	{
		self.set_edge_full(v, w, name, Some(label));
	}

	/// Equivalent to setEdge(v, w) with no label — applies default factory.
	pub fn set_edge_default(
		&mut self,
		v: impl Into<String>,
		w: impl Into<String>,
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
		v: impl Into<String>,
		w: impl Into<String>,
		name: Option<String>,
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
				self.edge_labels
					.insert(id.clone(), label);
			} else if let Some(f) = &self.default_edge_label {
				let e = Edge {
					v: v.clone(),
					w: w.clone(),
					name: name.clone(),
				};
				let l = f(&e);
				self.edge_labels.insert(id.clone(), l);
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
			name: name.clone(),
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
			.insert(id.clone(), edge);
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
			let (vv, ww) = (e.v.clone(), e.w.clone());
			self.edge_labels.remove(&id);
			self.edge_objs.remove(&id);
			if let Some(&idx) = self.edge_index.get(&id) {
				self.edge_order.remove(idx);
				self.edge_index.clear();
				for (i, eid) in self.edge_order.iter().enumerate() {
					self.edge_index.insert(eid.clone(), i);
				}
			}
			if let Some(m) = self.out_edges.get_mut(&vv) {
				m.remove(&id);
			}
			if let Some(m) = self.in_edges.get_mut(&ww) {
				m.remove(&id);
			}
			if let Some(m) = self.sucs.get_mut(&vv) {
				if let Some(cnt) = m.get_mut(&ww) {
					*cnt -= 1;
					if *cnt == 0 {
						m.remove(&ww);
					}
				}
			}
			if let Some(m) = self.preds.get_mut(&ww) {
				if let Some(cnt) = m.get_mut(&vv) {
					*cnt -= 1;
					if *cnt == 0 {
						m.remove(&vv);
					}
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
	pub fn predecessors(&self, v: &str) -> Option<Vec<String>> {
		let m = self.preds.get(v)?;
		Some(m.keys().cloned().collect())
	}
	pub fn successors(&self, v: &str) -> Option<Vec<String>> {
		let m = self.sucs.get(v)?;
		Some(m.keys().cloned().collect())
	}
	pub fn neighbors(&self, v: &str) -> Option<Vec<String>> {
		let mut s: Vec<String> = self.predecessors(v)?;
		let succ = self.successors(v)?;
		let set: HashSet<String> = s.iter().cloned().collect();
		for n in succ {
			if !set.contains(&n) {
				s.push(n);
			}
		}
		Some(s)
	}

	pub fn sources(&self) -> Vec<String> {
		self.node_order
			.iter()
			.filter(|v| {
				self.preds
					.get(*v)
					.map(|m| m.is_empty())
					.unwrap_or(true)
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
				path[i - 1].to_string(),
				path[i].to_string(),
				None,
				None,
			);
		}
	}

	pub fn sinks(&self) -> Vec<String> {
		self.node_order
			.iter()
			.filter(|v| {
				self.sucs
					.get(*v)
					.map(|m| m.is_empty())
					.unwrap_or(true)
			})
			.cloned()
			.collect()
	}
}

// ---- graph algorithms used by network-simplex --------------------------

pub mod alg {
	use super::Graph;
	use std::collections::HashSet;

	/// Postorder DFS traversal — used by network-simplex.
	pub fn postorder<G, N, E>(
		g: &Graph<G, N, E>,
		starts: &[String],
	) -> Vec<String> {
		let mut visited: HashSet<String> = HashSet::new();
		let mut result: Vec<String> = Vec::new();
		for s in starts {
			if !g.has_node(s) {
				panic!("postorder: node {} not in graph", s);
			}
			dfs(g, s, &mut visited, &mut result, true);
		}
		result
	}

	/// Tarjan-style strongly connected components on a directed graph; used
	/// by `find_cycles`. Only the SCCs with > 1 node (or self-loops) form
	/// cycles.
	pub fn tarjan<G, N, E>(g: &Graph<G, N, E>) -> Vec<Vec<String>> {
		struct State<'a, G: 'a, N: 'a, E: 'a> {
			g: &'a Graph<G, N, E>,
			index: usize,
			stack: Vec<String>,
			on_stack: std::collections::HashSet<String>,
			indices: std::collections::HashMap<String, usize>,
			lowlinks: std::collections::HashMap<String, usize>,
			results: Vec<Vec<String>>,
		}
		fn strong_connect<G, N, E>(s: &mut State<G, N, E>, v: &str) {
			s.indices.insert(v.to_string(), s.index);
			s.lowlinks
				.insert(v.to_string(), s.index);
			s.index += 1;
			s.stack.push(v.to_string());
			s.on_stack.insert(v.to_string());
			for w in s.g.successors(v).unwrap_or_default() {
				if !s.indices.contains_key(&w) {
					strong_connect(s, &w);
					let lw = *s.lowlinks.get(&w).unwrap();
					let lv = *s.lowlinks.get(v).unwrap();
					s.lowlinks
						.insert(v.to_string(), lv.min(lw));
				} else if s.on_stack.contains(&w) {
					let iw = *s.indices.get(&w).unwrap();
					let lv = *s.lowlinks.get(v).unwrap();
					s.lowlinks
						.insert(v.to_string(), lv.min(iw));
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
	pub fn find_cycles<G, N, E>(g: &Graph<G, N, E>) -> Vec<Vec<String>> {
		tarjan(g)
			.into_iter()
			.filter(|comp| {
				if comp.len() > 1 {
					true
				} else if comp.len() == 1 {
					g.has_edge(&comp[0], &comp[0])
				} else {
					false
				}
			})
			.collect()
	}

	/// Preorder DFS traversal.
	pub fn preorder<G, N, E>(
		g: &Graph<G, N, E>,
		starts: &[String],
	) -> Vec<String> {
		let mut visited: HashSet<String> = HashSet::new();
		let mut result: Vec<String> = Vec::new();
		for s in starts {
			if !g.has_node(s) {
				panic!("preorder: node {} not in graph", s);
			}
			dfs(g, s, &mut visited, &mut result, false);
		}
		result
	}

	fn dfs<G, N, E>(
		g: &Graph<G, N, E>,
		v: &str,
		visited: &mut HashSet<String>,
		result: &mut Vec<String>,
		postorder: bool,
	) {
		if visited.contains(v) {
			return;
		}
		visited.insert(v.to_string());
		if !postorder {
			result.push(v.to_string());
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
			result.push(v.to_string());
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
		assert_eq!(g.successors("a").unwrap(), vec!["b".to_string()]);
		assert_eq!(g.predecessors("b").unwrap(), vec!["a".to_string()]);
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

	#[test]
	fn compound_parent_children() {
		let mut g: Graph<(), i32, i32> =
			Graph::with_opts(GraphOpts::directed().compound());
		g.set_node("p", 0);
		g.set_node("c", 1);
		g.set_parent("c", Some("p"));
		assert_eq!(g.parent("c"), Some("p"));
		assert_eq!(g.children(Some("p")), vec!["c".to_string()]);
	}
}
