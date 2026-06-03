//! Top-level layout entry point. Wires together the pipeline:
//! acyclic -> rank -> normalize -> order -> position -> acyclic.undo.
//!
//! Scope note: this port intentionally omits the nesting-graph,
//! parent-dummy-chains, add-border-segments, coordinate-system, and
//! edge-label-proxy phases from `lib/layout.ts`. As a result it supports
//! non-compound, non-self-edge, non-self-loop, basic edge layouts only —
//! enough for typical small dag layouts.

use crate::{
	acyclic,
	graph::Graph,
	normalize,
	order,
	position,
	rank,
	types::{EdgeLabel, GraphLabel, NodeLabel},
	util,
};

#[derive(Debug, Default, Clone)]
pub struct LayoutOptions {
	pub disable_optimal_order_heuristic: bool,
}

pub fn layout(g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	layout_with(g, &LayoutOptions::default());
}

pub fn layout_with(
	g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
	opts: &LayoutOptions,
) {
	// acyclic reversal and normalize insert named edges, so the graph must
	// be a multigraph internally. Existing user edges have no name, so
	// flipping the flag never collides with what they already inserted.
	g.set_multigraph(true);
	make_space_for_edge_labels(g);
	let mut p = Profile::new();
	acyclic::run(g);
	p.tick("acyclic", g);
	rank::rank(g);
	p.tick("rank", g);
	util::normalize_ranks(g);
	p.tick("normalize_ranks", g);
	normalize::run(g);
	p.tick("normalize", g);
	order::order(
		g,
		&order::OrderOptions {
			disable_optimal_order_heuristic: opts
				.disable_optimal_order_heuristic,
		},
	);
	p.tick("order", g);
	position::position(g);
	p.tick("position", g);
	normalize::undo(g);
	p.tick("normalize::undo", g);
	translate_graph(g);
	p.tick("translate", g);
	acyclic::undo(g);
	p.tick("acyclic::undo", g);
}

#[cfg(feature = "profile")]
struct Profile {
	last: std::time::Instant,
}

#[cfg(feature = "profile")]
impl Profile {
	fn new() -> Self {
		Self {
			last: std::time::Instant::now(),
		}
	}
	fn tick(&mut self, phase: &str, g: &Graph<GraphLabel, NodeLabel, EdgeLabel>) {
		let now = std::time::Instant::now();
		let dt = now.duration_since(self.last);
		eprintln!(
			"[dagre] phase={:<16} ms={:>8.2} nodes={} edges={}",
			phase,
			dt.as_secs_f64() * 1000.0,
			g.node_count(),
			g.edge_count(),
		);
		self.last = now;
	}
}

#[cfg(not(feature = "profile"))]
struct Profile;

#[cfg(not(feature = "profile"))]
impl Profile {
	#[inline(always)]
	fn new() -> Self {
		Self
	}
	#[inline(always)]
	fn tick(
		&mut self,
		_phase: &str,
		_g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	) {
	}
}

/// Halves ranksep + doubles minlen, matching the JS makeSpaceForEdgeLabels.
fn make_space_for_edge_labels(g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	if let Some(gl) = g.graph_mut() {
		if let Some(r) = gl.ranksep.as_mut() {
			*r /= 2.0;
		}
	}
	let edges = g.edges();
	for e in edges {
		if let Some(label) = g.edge_obj_mut(&e) {
			label.minlen *= 2;
		}
	}
}

/// Shift all coordinates so the layout origin is (margin, margin).
fn translate_graph(g: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>) {
	let mut min_x = f64::INFINITY;
	let mut max_x = f64::NEG_INFINITY;
	let mut min_y = f64::INFINITY;
	let mut max_y = f64::NEG_INFINITY;
	let (mx, my) = {
		let gl = g.graph().cloned().unwrap_or_default();
		(gl.marginx.unwrap_or(0.0), gl.marginy.unwrap_or(0.0))
	};

	for v in g.nodes() {
		if let Some(n) = g.node(&v) {
			if let (Some(x), Some(y)) = (n.x, n.y) {
				min_x = min_x.min(x - n.width / 2.0);
				max_x = max_x.max(x + n.width / 2.0);
				min_y = min_y.min(y - n.height / 2.0);
				max_y = max_y.max(y + n.height / 2.0);
			}
		}
	}
	if !min_x.is_finite() {
		return;
	}
	min_x -= mx;
	min_y -= my;
	for v in g.nodes() {
		if let Some(n) = g.node_mut(&v) {
			if let Some(x) = n.x.as_mut() {
				*x -= min_x;
			}
			if let Some(y) = n.y.as_mut() {
				*y -= min_y;
			}
		}
	}
	for e in g.edges() {
		if let Some(l) = g.edge_obj_mut(&e) {
			if let Some(pts) = l.points.as_mut() {
				for p in pts.iter_mut() {
					p.x -= min_x;
					p.y -= min_y;
				}
			}
			if let Some(x) = l.x.as_mut() {
				*x -= min_x;
			}
			if let Some(y) = l.y.as_mut() {
				*y -= min_y;
			}
		}
	}
	if let Some(gl) = g.graph_mut() {
		gl.width = Some(max_x - min_x + mx);
		gl.height = Some(max_y - min_y + my);
	}
}
