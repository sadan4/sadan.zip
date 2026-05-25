#![expect(clippy::multiple_inherent_impl)]
use std::{
	collections::{HashMap, HashSet},
	iter,
	mem,
};

use crate::{Edge, EdgeLabelFactory, GraphOptions, NodeLabelFactory};

pub struct Graph<GraphLabel, NodeLabel, EdgeLabel> {
	is_directed: bool,
	is_multigraph: bool,
	is_compound: bool,
	/// Label for the graph itself
	label: Option<GraphLabel>,
	/// v -> label
	nodes: HashMap<String, NodeLabel>,
	/// v -> edgeObj
	in_: HashMap<String, HashMap<String, Edge>>,
	/// u -> v -> Number
	preds: HashMap<String, HashMap<String, u32>>,
	/// v -> edgeObj
	out: HashMap<String, HashMap<String, Edge>>,
	/// v -> w -> Number
	sucs: HashMap<String, HashMap<String, u32>>,
	/// e -> edgeObj
	edge_objs: HashMap<String, Edge>,
	/// e -> label
	edge_labels: HashMap<String, EdgeLabel>,
	/// Number of nodes in the graph. Should only be changed by the implementation.
	node_count: u32,
	/// Number of edges in the graph. Should only be changed by the implementation.
	edge_count: u32,
	parent: Option<HashMap<String, String>>,
	children: Option<HashMap<String, HashMap<String, bool>>>,
	default_node_label_fn: Option<Box<dyn NodeLabelFactory<NodeLabel>>>,
	default_edge_label_fn: Option<Box<dyn EdgeLabelFactory<EdgeLabel>>>,
}

const DEFAULT_EDGE_NAME: &str = "\x00";
const GRAPH_NODE: &str = "\x00";
const EDGE_KEY_DELIM: &str = "\x01";

impl<GraphLabel, NodeLabel, EdgeLabel> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	fn default_() -> Self {
		Self {
			is_directed: true,
			is_multigraph: false,
			is_compound: false,
			label: None,
			nodes: HashMap::default(),
			in_: HashMap::default(),
			preds: HashMap::default(),
			out: HashMap::default(),
			sucs: HashMap::default(),
			edge_objs: HashMap::default(),
			edge_labels: HashMap::default(),
			node_count: 0,
			edge_count: 0,
			parent: None,
			children: None,
			default_node_label_fn: None,
			default_edge_label_fn: None,
		}
	}
	pub fn new(options: GraphOptions) -> Self {
		let mut graph = Self {
			is_directed: options.directed,
			is_multigraph: options.multigraph,
			is_compound: options.compound,
			..Self::default_()
		};
		if graph.is_compound {
			// v -> parent
			graph.parent = Some(HashMap::new());
			// v -> children
			graph.children = Some(HashMap::new());
			graph
				.children
				.as_mut()
				.unwrap()
				.insert(GRAPH_NODE.to_string(), HashMap::new());
		}
		graph
	}
	/// Whether graph was created with 'directed' flag set to true or not.
	///
	/// @returns whether the graph edges have an orientation.
	pub fn is_directed(&self) -> bool {
		self.is_directed
	}

	/// Whether graph was created with 'multigraph' flag set to true or not.
	///
	/// @returns whether the pair of nodes of the graph can have multiple edges.
	pub fn is_multigraph(&self) -> bool {
		self.is_multigraph
	}
}

/// Graph functions
impl<GraphLabel, NodeLabel, EdgeLabel> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	/// Whether graph was created with 'compound' flag set to true or not.
	///
	/// @returns whether a node of the graph can have subnodes.
	pub fn is_compound(&self) -> bool {
		self.is_compound
	}
	/// Sets the label of the graph.
	///
	/// @param label - label value.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_graph(&mut self, label: GraphLabel) {
		self.label = Some(label);
	}

	/// Gets the graph label.
	///
	/// @returns currently assigned label for the graph or undefined if no label assigned.
	pub fn graph(&self) -> Option<&GraphLabel> {
		self.label.as_ref()
	}

	/// Sets the default node label. This label will be assigned as default label
	/// in case if no label was specified while setting a node.
	/// Complexity: O(1).
	///
	/// @param labelOrFn - default node label or label factory function.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_default_node_label(
		&mut self,
		label_factory: impl NodeLabelFactory<NodeLabel> + 'static,
	) {
		self.default_node_label_fn = Some(Box::new(label_factory));
	}

	/// Gets the number of nodes in the graph.
	/// Complexity: O(1).
	///
	/// @returns nodes count.
	pub fn node_count(&self) -> u32 {
		self.node_count
	}
}

/// Node functions
impl<GraphLabel, NodeLabel, EdgeLabel> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	/// Gets all nodes of the graph. Note, the in case of compound graph subnodes are
	/// not included in list.
	/// Complexity: O(1).
	///
	/// @returns list of graph nodes.
	pub fn nodes(&self) -> Vec<String> {
		self.nodes.keys().cloned().collect()
	}
	/// Gets list of nodes without in-edges.
	/// Complexity: O(|V|).
	///
	/// @returns the graph source nodes.
	pub fn sources(&self) -> Vec<String> {
		self.nodes()
			.into_iter()
			.filter(|v| self.in_[v].is_empty())
			.collect()
	}

	/// Gets list of nodes without out-edges.
	/// Complexity: O(|V|).
	///
	/// @returns the graph sink nodes.
	pub fn sinks(&self) -> Vec<String> {
		self.nodes()
			.into_iter()
			.filter(|v| self.out[v].is_empty())
			.collect()
	}

	/// Invokes setNode method for each node in names list.
	/// Complexity: O(|names|).
	///
	/// @param names - list of nodes names to be set.
	/// @param label - value to set for each node in list.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_nodes(&mut self, names: Vec<String>, label: Option<NodeLabel>)
	where
		NodeLabel: Clone,
	{
		for v in names {
			self.set_node(v.clone(), label.clone());
			// if label.is_some() {
			// 	self.set_node(v.clone(), Some(label.unwrap().clone()))
			// } else {
			// 	self.set_node(v.clone(), None)
			// }
		}
	}

	/// Creates or updates the value for the node v in the graph. If label is supplied
	/// it is set as the value for the node. If label is not supplied and the node was
	/// created by this call then the default node label will be assigned.
	/// Complexity: O(1).
	///
	/// @param name - node name.
	/// @param label - value to set for node.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_node(&mut self, v: String, label: impl Into<Option<NodeLabel>>) {
		let label = label.into();
		if self.nodes.contains_key(&v) {
			if let Some(label) = label {
				*self.nodes.get_mut(&v).unwrap() = label;
			}
			return;
		}
		self.nodes.insert(
			v.clone(),
			if let Some(label) = label {
				label
			} else if let Some(label_fn) = &self.default_node_label_fn {
				label_fn.create_node_label(v.clone())
			} else {
				panic!(
					"Node label required but not provided or default node label function set"
				)
			},
		);

		if self.is_compound {
			self.parent
				.as_mut()
				.unwrap()
				.insert(v.clone(), GRAPH_NODE.to_string());
			let children = self.children.as_mut().unwrap();
			children.insert(v.clone(), HashMap::new());
			children
				.get_mut(GRAPH_NODE)
				.unwrap()
				.insert(v.clone(), true);
		}
		self.in_
			.insert(v.clone(), HashMap::new());
		self.preds
			.insert(v.clone(), HashMap::new());
		self.out
			.insert(v.clone(), HashMap::new());
		self.sucs
			.insert(v.clone(), HashMap::new());
		self.node_count += 1;
	}

	/// Gets the label of node with specified name.
	/// Complexity: O(|V|).
	///
	/// @param name - node name.
	/// @returns label value of the node.
	pub fn node(&self, v: String) -> Option<NodeLabel>
	where
		NodeLabel: Clone,
	{
		self.nodes.get(&v).cloned()
	}

	/// Gets the label of node with specified name.
	/// Complexity: O(|V|).
	///
	/// @param name - node name.
	/// @returns label value of the node.
	pub fn node_mut(&mut self, v: String) -> Option<&mut NodeLabel>
	where
		NodeLabel: Clone,
	{
		self.nodes.get_mut(&v)
	}

	/// Detects whether graph has a node with specified name or not.
	///
	/// @param name - name of the node.
	/// @returns true if graph has node with specified name, false - otherwise.
	pub fn has_node(&self, v: String) -> bool {
		self.nodes.contains_key(&v)
	}

	pub fn remove_node(&mut self, v: String) {
		if !self.nodes.contains_key(&v) {
			return;
		}
		// let remove_edge =
		// 	|e: String| self.remove_edge_by_obj(self.edge_objs[&e].clone());
		macro_rules! remove_edge {
			($e:expr) => {
				self.remove_edge_by_obj(self.edge_objs[&$e].clone())
			};
		}
		self.nodes.remove(&v);
		if self.is_compound {
			self.remove_from_parents_child_list(v.clone());
			self.parent.as_mut().unwrap().remove(&v);
			for child in self.children(v.clone()) {
				self.set_parent(child, None);
			}
			self.children
				.as_mut()
				.unwrap()
				.remove(&v);
		}
		if let Some(in_) = self.in_.get(&v) {
			let in_: Vec<_> = in_.keys().cloned().collect();
			for edge in in_ {
				remove_edge!(edge);
			}
		}
		self.in_.remove(&v);
		self.preds.remove(&v);
		if let Some(out) = self.out.get(&v) {
			let out: Vec<_> = out.keys().cloned().collect();
			for edge in out {
				remove_edge!(edge);
			}
		}
		self.out.remove(&v);
		self.sucs.remove(&v);
		self.node_count -= 1;
	}

	/// Sets node parent for node v if it is defined, or removes the
	/// parent for v if p is undefined. Method throws an exception in case of
	/// invoking it in context of noncompound graph.
	/// Average-case complexity: O(1).
	///
	/// @param v - node to be child for p.
	/// @param parent - node to be parent for v.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_parent(&mut self, v: String, parent: impl Into<Option<String>>) {
		let parent = parent.into();
		if self.is_compound {
			panic!("Cannot set parent in a non-compound graph");
		}

		let parent = if parent == None {
			GRAPH_NODE.to_string()
		} else {
			let parent = parent.unwrap();
			let mut ancestor: String = parent.clone();

			loop {
				if ancestor == v {
					panic!(
						"Setting {parent} as parent of {v} would create a cycle"
					);
				}
				if let Some(next) = self.parent(ancestor.clone()) {
					ancestor = next;
				} else {
					break;
				}
			}

			self.set_node(parent.clone(), None);
			parent
		};
		self.set_node(v.clone(), None);
		self.remove_from_parents_child_list(v.clone());
		self.parent
			.as_mut()
			.unwrap()
			.insert(v.clone(), parent.clone());
		self.children
			.as_mut()
			.unwrap()
			.get_mut(&parent)
			.unwrap()
			.insert(v.clone(), true);
	}

	/// Gets parent node for node v.
	/// Complexity: O(1).
	///
	/// @param v - node to get parent of.
	/// @returns parent node name or void if v has no parent.
	pub fn parent(&self, v: String) -> Option<String> {
		if self.is_compound {
			let parent = self.parent.as_ref().unwrap().get(&v)?;
			if parent != &GRAPH_NODE {
				return Some(parent.clone());
			}
		}
		None
	}

	/// Gets list of direct children of node v.
	/// Complexity: O(1).
	///
	/// @param v - node to get children of.
	/// @returns children nodes names list.
	pub fn children(&self, v: impl Into<Option<String>>) -> Vec<String> {
		let v = v.into();
		let v = v.unwrap_or_else(|| GRAPH_NODE.to_string());
		if self.is_compound {
			let children = self.children.as_ref().unwrap().get(&v);
			if let Some(children) = children {
				return children.keys().cloned().collect();
			}
		} else if v == GRAPH_NODE {
			return self.nodes();
		} else if self.has_node(v.clone()) {
			return vec![];
		}
		return vec![];
	}

	/// Return all nodes that are predecessors of the specified node or undefined if node v is not in
	/// the graph. Behavior is undefined for undirected graphs - use neighbors instead.
	/// Complexity: O(|V|).
	///
	/// @param v - node identifier.
	/// @returns node identifiers list or undefined if v is not in the graph.
	pub fn predecessors(&self, v: String) -> Option<Vec<String>> {
		let preds_v = self.preds.get(&v);
		if let Some(preds_v) = preds_v {
			return Some(preds_v.keys().cloned().collect());
		}
		return None;
	}

	/// Return all nodes that are successors of the specified node or undefined if node v is not in
	/// the graph. Behavior is undefined for undirected graphs - use neighbors instead.
	/// Complexity: O(|V|).
	///
	/// @param v - node identifier.
	/// @returns node identifiers list or undefined if v is not in the graph.
	pub fn successors(&self, v: String) -> Option<Vec<String>> {
		let sucs_v = self.sucs.get(&v);
		if let Some(sucs_v) = sucs_v {
			return Some(sucs_v.keys().cloned().collect());
		}
		return None;
	}
	/// Return all nodes that are predecessors or successors of the specified node or undefined if
	/// node v is not in the graph.
	/// Complexity: O(|V|).
	///
	/// @param v - node identifier.
	/// @returns node identifiers list or undefined if v is not in the graph.
	pub fn neighbors(&self, v: String) -> Option<Vec<String>> {
		let preds = self.predecessors(v.clone())?;
		let mut unique: HashSet<_> = preds.into_iter().collect();
		if let Some(sucs) = self.successors(v.clone()) {
			for succ in sucs {
				unique.insert(succ);
			}
		}
		Some(unique.into_iter().collect())
	}

	pub fn is_leaf(&self, v: String) -> bool {
		let neighbors = if self.is_directed {
			self.successors(v)
		} else {
			self.neighbors(v)
		};
		neighbors.unwrap().len() == 0
	}

	/// Creates new graph with nodes filtered via filter. Edges incident to rejected node
	/// are also removed. In case of compound graph, if parent is rejected by filter,
	/// than all its children are rejected too.
	/// Average-case complexity: O(|E|+|V|).
	///
	/// @param filter - filtration function detecting whether the node should stay or not.
	/// @returns new graph made from current and nodes filtered.
	#[must_use]
	pub fn filter_nodes(&self, filter: impl Fn(&str) -> bool) -> Self
	where
		GraphLabel: Clone,
		NodeLabel: Clone,
		EdgeLabel: Clone,
	{
		let mut copy = Self::new(GraphOptions {
			directed: self.is_directed,
			multigraph: self.is_multigraph,
			compound: self.is_compound,
		});

		if let Some(graph) = self.graph().cloned() {
			copy.set_graph(graph);
		}
		for (v, value) in &self.nodes {
			if filter(v) {
				copy.set_node(v.clone(), value.clone());
			}
		}
		for e in self.edge_objs.values() {
			if copy.has_node(e.v.clone()) && copy.has_node(e.w.clone()) {
				copy.set_edge_from_obj(
					e.clone(),
					self.edge_from_obj(e.clone()),
				);
			}
		}
		let mut parents = HashMap::new();
		fn find_parent<GraphLabel, NodeLabel, EdgeLabel>(
			this: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
			copy: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
			parents: &mut HashMap<String, Option<String>>,
			v: String,
		) -> Option<String> {
			let parent = this.parent(v.clone());

			if parent.is_none() {
				parents.insert(v.clone(), None);
				return None;
			}

			let parent = parent.unwrap();

			if parent.is_empty() || copy.has_node(parent.clone()) {
				parents.insert(v.clone(), Some(parent.clone()));
				return Some(parent);
			} else if let Some(parent) = parents.get(&parent) {
				return parent.clone();
			}

			return find_parent(this, copy, parents, parent.clone());
		}

		if self.is_compound {
			for v in copy.nodes() {
				copy.set_parent(
					v.clone(),
					find_parent(self, &copy, &mut parents, v.clone()),
				)
			}
		}

		copy
	}

	/// Sets the default edge label. This label will be assigned as default label
	/// in case if no label was specified while setting an edge.
	/// Complexity: O(1).
	///
	/// @param labelOrFn - default edge label or label factory function.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_default_edge_label(
		&mut self,
		label_factory: impl EdgeLabelFactory<EdgeLabel> + 'static,
	) {
		self.default_edge_label_fn = Some(Box::new(label_factory));
	}

	/// Gets the number of edges in the graph.
	/// Complexity: O(1).
	///
	/// @returns edges count.
	pub fn edge_count(&self) -> u32 {
		self.edge_count
	}

	/// Gets edges of the graph. In case of compound graph subgraphs are not considered.
	/// Complexity: O(|E|).
	///
	/// @returns graph edges list.
	pub fn edges(&self) -> Vec<Edge> {
		self.edge_objs
			.values()
			.cloned()
			.collect()
	}
}
/// Edge functions
impl<GraphLabel, NodeLabel, EdgeLabel> Graph<GraphLabel, NodeLabel, EdgeLabel> {
	/// Establish an edges path over the nodes in nodes list. If some edge is already
	/// exists, it will update its label, otherwise it will create an edge between pair
	/// of nodes with label provided or default label if no label provided.
	/// Complexity: O(|nodes|).
	///
	/// @param nodes - list of nodes to be connected in series.
	/// @param label - value to set for each edge between pairs of nodes.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_path(
		&mut self,
		nodes: Vec<String>,
		label: impl Into<Option<EdgeLabel>>,
	) where
		EdgeLabel: Clone,
	{
		let label = label.into();
		nodes.into_iter().reduce(|v, w| {
			self.set_edge(v.clone(), w.clone(), label.clone(), None);
			w
		});
	}

	/// Creates or updates the label for the specified edge. If label is supplied it is
	/// set as the value for the edge. If label is not supplied and the edge was created
	/// by this call then the default edge label will be assigned. The name parameter is
	/// only useful with multigraphs.
	/// Complexity: O(1).
	///
	/// @param edge - edge descriptor.
	/// @param label - value to associate with the edge.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_edge_from_obj(
		&mut self,
		edge: Edge,
		label: impl Into<Option<EdgeLabel>>,
	) {
		self.set_edge(edge.v, edge.w, label, edge.name);
	}

	/// Creates or updates the label for the edge (v, w) with the optionally supplied
	/// name. If label is supplied it is set as the value for the edge. If label is not
	/// supplied and the edge was created by this call then the default edge label will
	/// be assigned. The name parameter is only useful with multigraphs.
	/// Complexity: O(1).
	///
	/// @param v - edge source node.
	/// @param w - edge sink node.
	/// @param label - value to associate with the edge.
	/// @param name - unique name of the edge in order to identify it in multigraph.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn set_edge(
		&mut self,
		v: String,
		w: String,
		label: impl Into<Option<EdgeLabel>>,
		name: impl Into<Option<String>>,
	) {
		let label = label.into();
		let name = name.into();
		let v_str = v;
		let w_str = w;
		let name_str = name;
		let edge_value = label;
		let value_specified = edge_value.is_some();

		let e = edge_args_to_id(
			self.is_directed,
			v_str.clone(),
			w_str.clone(),
			name_str.clone(),
		);

		if self.edge_labels.contains_key(&e) {
			if let Some(value) = edge_value {
				*self.edge_labels.get_mut(&e).unwrap() = value;
			}
			return;
		}

		// It didn't exist, so we need to create it.
		// First ensure the nodes exist.
		self.set_node(v_str.clone(), None);
		self.set_node(w_str.clone(), None);

		self.edge_labels.insert(
			e.clone(),
			edge_value.unwrap_or_else(|| {
				self.default_edge_label_fn
					.as_ref()
					.expect(
						"no label passed and no default edge label factory provided",
					)
					.create_edge_label(
						v_str.clone(),
						w_str.clone(),
						name_str.clone(),
					)
			}),
		);

		// Ensure we add undirected edges in a consistent way.
		let edge_obj = edge_args_to_obj(
			self.is_directed,
			v_str.clone(),
			w_str.clone(),
			name_str.clone(),
		);

		let v_str = edge_obj.v.clone();
		let w_str = edge_obj.w.clone();

		self.edge_objs
			.insert(e.clone(), edge_obj.clone());
		increment_or_init_entry(
			self.preds.get_mut(&w_str).unwrap(),
			v_str.clone(),
		);
		increment_or_init_entry(
			self.sucs.get_mut(&v_str).unwrap(),
			w_str.clone(),
		);
		self.in_
			.get_mut(&w_str)
			.unwrap()
			.insert(e.clone(), edge_obj.clone());
		self.out
			.get_mut(&v_str)
			.unwrap()
			.insert(e.clone(), edge_obj.clone());
		self.edge_count += 1;
	}

	/// Gets the label for the specified edge.
	/// Complexity: O(1).
	///
	/// @param edge - edge descriptor.
	/// @returns value associated with specified edge.
	pub fn edge_from_obj(&self, edge: Edge) -> Option<EdgeLabel>
	where
		EdgeLabel: Clone,
	{
		self.edge(edge.v, edge.w, edge.name)
	}

	/// Gets the label for the specified edge.
	/// Complexity: O(1).
	///
	/// @param v - edge source node.
	/// @param w - edge sink node.
	/// @param name - name of the edge (actual for multigraph).
	/// @returns value associated with specified edge.
	pub fn edge(
		&self,
		v: String,
		w: String,
		name: Option<String>,
	) -> Option<EdgeLabel>
	where
		EdgeLabel: Clone,
	{
		let e = edge_args_to_id(self.is_directed, v, w, name);
		self.edge_labels.get(&e).cloned()
	}

	/// Detects whether the graph contains specified edge or not. No subgraphs are considered.
	/// Complexity: O(1).
	///
	/// @param edge - edge descriptor.
	/// @returns whether the graph contains the specified edge or not.
	pub fn has_edge_by_obj(&self, edge: Edge) -> bool {
		self.has_edge(edge.v, edge.w, edge.name)
	}

	/// Gets the label for the specified edge.
	/// Complexity: O(1).
	///
	/// @param edge - edge descriptor.
	/// @returns value associated with specified edge.
	pub fn has_edge(&self, v: String, w: String, name: Option<String>) -> bool {
		let e = edge_args_to_id(self.is_directed, v, w, name);
		self.edge_labels.contains_key(&e)
	}

	/// Removes the specified edge from the graph. No subgraphs are considered.
	/// Complexity: O(1).
	///
	/// @param edge - edge descriptor.
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn remove_edge_by_obj(&mut self, edge: Edge) {
		self.remove_edge(edge.v, edge.w, edge.name);
	}

	/// Removes the specified edge from the graph. No subgraphs are considered.
	/// Complexity: O(1).
	///
	/// @param v - edge source node.
	/// @param w - edge sink node.
	/// @param name - name of the edge (actual for multigraph).
	/// @returns the graph, allowing this to be chained with other functions.
	pub fn remove_edge(&mut self, v: String, w: String, name: Option<String>) {
		let e = edge_args_to_id(self.is_directed, v, w, name);
		let edge = self.edge_objs.get(&e).cloned();
		if let Some(edge) = edge {
			let v_str = edge.v;
			let w_str = edge.w;
			self.edge_labels.remove(&e);
			self.edge_objs.remove(&e);
			decrement_or_remove_entry(
				self.preds.get_mut(&w_str).unwrap(),
				v_str.clone(),
			);
			decrement_or_remove_entry(
				self.sucs.get_mut(&v_str).unwrap(),
				w_str.clone(),
			);
			self.in_
				.get_mut(&w_str)
				.unwrap()
				.remove(&e);
			self.out
				.get_mut(&v_str)
				.unwrap()
				.remove(&e);
			self.edge_count -= 1;
		}
	}

	/// Return all edges that point to the node v. Optionally filters those edges down to just those
	/// coming from node u. Behavior is void for undirected graphs - use nodeEdges instead.
	/// Complexity: O(|E|).
	///
	/// @param v - edge sink node.
	/// @param w - edge source node.
	/// @returns edges descriptors list if v is in the graph, or void otherwise.
	pub fn in_edges(
		&self,
		v: String,
		w: impl Into<Option<String>>,
	) -> Option<Vec<Edge>> {
		let w = w.into();
		if self.is_directed {
			filter_edges(self.in_.get(&v)?.values(), v, w).into()
		} else {
			self.node_edges(v, w)
		}
	}

	/// Return all edges that are pointed at by node v. Optionally filters those edges down to just
	/// those point to w. Behavior is void for undirected graphs - use nodeEdges instead.
	/// Complexity: O(|E|).
	///
	/// @param v - edge source node.
	/// @param w - edge sink node.
	/// @returns edges descriptors list if v is in the graph, or void otherwise.
	pub fn out_edges(
		&self,
		v: String,
		w: impl Into<Option<String>>,
	) -> Option<Vec<Edge>> {
		let w = w.into();
		if self.is_directed {
			filter_edges(self.out.get(&v)?.values(), v, w).into()
		} else {
			self.node_edges(v, w)
		}
	}

	/// Returns all edges to or from node v regardless of direction. Optionally filters those edges
	/// down to just those between nodes v and w regardless of direction.
	/// Complexity: O(|E|).
	///
	/// @param v - edge adjacent node.
	/// @param w - edge adjacent node.
	/// @returns edges descriptors list if v is in the graph, or void otherwise.
	pub fn node_edges(
		&self,
		v: String,
		w: impl Into<Option<String>>,
	) -> Option<Vec<Edge>> {
		let w = w.into();
		if self.nodes.contains_key(&v) {
			filter_edges(
				iter::chain(self.in_[&v].values(), self.out[&v].values()),
				v,
				w,
			)
			.into()
		} else {
			None
		}
	}

	fn remove_from_parents_child_list(&mut self, v: String) {
		self.children
			.as_mut()
			.unwrap()
			.get_mut(&self.parent.as_ref().unwrap()[&v])
			.unwrap()
			.remove(&v);
	}
}
fn edge_args_to_id(
	is_directed: bool,
	mut v: String,
	mut w: String,
	name: Option<String>,
) -> String {
	if !is_directed && v > w {
		mem::swap(&mut v, &mut w);
	}
	format!(
		"{v}{EDGE_KEY_DELIM}{w}{EDGE_KEY_DELIM}{}",
		name.as_ref()
			.map(String::as_str)
			.unwrap_or(DEFAULT_EDGE_NAME)
	)
}

fn edge_args_to_obj(
	is_directed: bool,
	mut v: String,
	mut w: String,
	name: Option<String>,
) -> Edge {
	if !is_directed && v > w {
		mem::swap(&mut v, &mut w);
	}
	Edge { v, w, name }
}

fn increment_or_init_entry(map: &mut HashMap<String, u32>, k: String) {
	if map.contains_key(&k) {
		*map.get_mut(&k).unwrap() += 1;
	} else {
		map.insert(k, 1);
	}
}

fn decrement_or_remove_entry(map: &mut HashMap<String, u32>, k: String) {
	if let Some(count) = map.get_mut(&k) {
		if *count == 1 {
			map.remove(&k);
		} else {
			*count -= 1;
		}
	}
}

fn filter_edges<'a>(
	edges: impl Iterator<Item = &'a Edge>,
	local_edge: String,
	remote_edge: Option<String>,
) -> Vec<Edge> {
	if remote_edge.is_none() {
		edges.cloned().collect()
	} else {
		let remote_edge = remote_edge.unwrap();
		edges
			.filter(|edge| {
				(edge.v == local_edge && edge.w == remote_edge)
					|| (edge.v == remote_edge && edge.w == local_edge)
			})
			.cloned()
			.collect()
	}
}
