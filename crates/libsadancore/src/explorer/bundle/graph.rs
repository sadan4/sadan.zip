use dagre::{EdgeLabel, GraphLabel, NodeLabel, RankDir, layout};
use explorer_types::ModuleId;
use std::{collections::HashSet, iter, mem};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{explorer::bundle::Bundle, util::console_log};

type Graph = dagre::Graph<GraphLabel, NodeLabel, EdgeLabel>;

#[wasm_bindgen]
#[derive(Copy, Clone)]
pub struct JSEdge {
	from: ModuleId,
	to: ModuleId,
}

#[wasm_bindgen]
impl JSEdge {
	#[wasm_bindgen(getter)]
	#[expect(
		clippy::trivially_copy_pass_by_ref,
		reason = "wasm_bindgen moves Copy structs"
	)]
	pub fn from(&self) -> u32 {
		*self.from
	}
	#[wasm_bindgen(getter)]
	#[expect(
		clippy::trivially_copy_pass_by_ref,
		reason = "wasm_bindgen moves Copy structs"
	)]
	pub fn to(&self) -> u32 {
		*self.to
	}
}

#[wasm_bindgen]
#[derive(Copy, Clone)]
pub struct JSNode {
	id: ModuleId,
	pub width: f64,
	pub height: f64,
	pub x: f64,
	pub y: f64,
}

#[wasm_bindgen]
impl JSNode {
	#[wasm_bindgen(getter)]
	pub fn id(&self) -> u32 {
		*self.id
	}
}

#[wasm_bindgen]
pub struct LaidOutGraph {
	nodes: Box<[JSNode]>,
	edges: Box<[JSEdge]>,
}

#[wasm_bindgen]
impl LaidOutGraph {
	#[wasm_bindgen(getter)]
	pub fn nodes(&self) -> Box<[JSNode]> {
		self.nodes.clone()
	}
	#[wasm_bindgen(getter)]
	pub fn edges(&self) -> Box<[JSEdge]> {
		self.edges.clone()
	}
}

#[wasm_bindgen]
#[expect(clippy::multiple_inherent_impl)]
impl Bundle {
	/// Depth defaults to 1.
	#[wasm_bindgen]
	pub fn gen_graph(&self, module_id: u32, depth: Option<u8>) -> LaidOutGraph {
		let init_depth = depth.unwrap_or(1);
		let mut depth = init_depth;
		let module_id = ModuleId(module_id);
		let mut graph = Graph::new();
		graph.set_default_node_label(|_| NodeLabel {
			width: 75.,
			height: 45.,
			..NodeLabel::default()
		});
		graph.set_graph(GraphLabel {
			rankdir: Some(RankDir::TB),
			..GraphLabel::default()
		});

		let mut new_q: Vec<ModuleId> = Vec::new();
		let mut q: Vec<ModuleId> = vec![module_id];
		let mut included_nodes = HashSet::from([module_id]);
		console_log!("probing down");
		// probe down
		while depth != 0 {
			depth -= 1;
			debug_assert!(new_q.is_empty());
			for m_id in q.drain(..) {
				let parser = self
					.inner
					.get_or_make_parser(m_id)
					.expect("Failed to get parser");
				let deps = parser
					.get_modules_that_this_module_requires()
					.unwrap_or_default();
				for dep in iter::chain(deps.sync, deps.lazy) {
					included_nodes.insert(dep);
					new_q.push(dep);
				}
			}
			mem::swap(&mut q, &mut new_q);
		}
		// reset depth
		depth = init_depth;
		// reset queue
		debug_assert!(new_q.is_empty());
		q.clear();
		q.push(module_id);
		console_log!("probing up");
		// probe up
		while depth != 0 {
			depth -= 1;
			debug_assert!(new_q.is_empty());
			for m_id in q.drain(..) {
				let Some(incoming) = self
					.inner
					.dep_info
					.module_deps
					.get(&m_id)
				else {
					continue;
				};
				for dep in iter::chain(&incoming.sync, &incoming.lazy) {
					included_nodes.insert(*dep);
					new_q.push(*dep);
				}
			}
			mem::swap(&mut q, &mut new_q);
		}
		console_log!("adding {} nodes", included_nodes.len());
		for m_id in &included_nodes {
			graph.set_node(
				format!("{m_id}"),
				NodeLabel {
					width: 75.,
					height: 40.,
					..NodeLabel::default()
				},
			);
		}
		console_log!("adding edges");
		for (m_id, dep_info) in &self.inner.dep_info.module_deps {
			if !included_nodes.contains(m_id) {
				continue;
			}
			for dependent in iter::chain(&dep_info.sync, &dep_info.lazy) {
				if !included_nodes.contains(dependent) {
					continue;
				}
				// dependent -> m_id (dependent requires this module)
				graph.set_edge(
					format!("{dependent}"),
					format!("{m_id}"),
					EdgeLabel::default(),
				);
			}
		}
		console_log!("added {} edges", graph.edge_count());
		console_log!("laying out graph");
		layout(&mut graph);
		console_log!("done laying out graph");
		let nodes = graph
			.nodes()
			.into_iter()
			.map(|id| {
				let label = graph.node(&id).unwrap();
				(id, label)
			})
			.map(|(id, label)| JSNode {
				id: ModuleId(id.parse().unwrap()),
				width: label.width,
				height: label.height,
				x: label.x.unwrap_or_default(),
				y: label.y.unwrap_or_default(),
			})
			.collect();
		let edges = graph
			.edges()
			.into_iter()
			.map(|edge| JSEdge {
				from: ModuleId(edge.v.parse().unwrap()),
				to: ModuleId(edge.w.parse().unwrap()),
			})
			.collect();

		LaidOutGraph { nodes, edges }
	}
}
