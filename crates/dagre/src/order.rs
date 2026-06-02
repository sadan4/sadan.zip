//! Crossing-minimization order pipeline — port of `lib/order/*.ts`.

use crate::graph::{Graph, GraphOpts};
use crate::types::{EdgeLabel, GraphLabel, NodeLabel};
use crate::util;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct OrderOptions {
    pub disable_optimal_order_heuristic: bool,
}

#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
struct LayerNode {
    // rank/min_rank/max_rank carried for parity with JS layer-graph node label;
    // ordering only consults `order` and borders.
    rank: Option<i32>,
    min_rank: Option<i32>,
    max_rank: Option<i32>,
    order: Option<usize>,
    border_left: Option<String>,
    border_right: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct LayerEdge {
    weight: f64,
}

#[derive(Debug, Clone)]
struct LayerGraphLabel {
    root: String,
}

type LayerGraph = Graph<LayerGraphLabel, LayerNode, LayerEdge>;

#[derive(Debug, Clone, PartialEq)]
pub struct BarycenterEntry {
    pub v: String,
    pub barycenter: Option<f64>,
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEntry {
    pub vs: Vec<String>,
    pub i: usize,
    pub barycenter: Option<f64>,
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SortResult {
    pub vs: Vec<String>,
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
    movable: &[String],
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
                    let edge_w = graph.edge_obj(&e).map(|l| l.weight).unwrap_or(0.0);
                    let order = graph
                        .node(&e.v)
                        .and_then(|n| n.order)
                        .unwrap_or(0) as f64;
                    sum += edge_w * order;
                    weight += edge_w;
                }
                BarycenterEntry {
                    v: v.clone(),
                    barycenter: Some(if weight > 0.0 { sum / weight } else { 0.0 }),
                    weight: Some(weight),
                }
            }
        })
        .collect()
}

/// Public resolve-conflicts: takes a list of barycenter entries and a
/// constraint graph, returns coalesced entries respecting the constraints.
pub fn resolve_conflicts(
    entries: Vec<BarycenterEntry>,
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
    vs: &[String],
) {
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut root_prev: Option<String> = None;
    for v in vs {
        let mut child = graph.parent(v).map(|s| s.to_string());
        while let Some(c) = child.clone() {
            let parent = graph.parent(&c).map(|s| s.to_string());
            let prev_child = match &parent {
                Some(p) => {
                    let pc = prev.get(p).cloned();
                    prev.insert(p.clone(), c.clone());
                    pc
                }
                None => {
                    let pc = root_prev.clone();
                    root_prev = Some(c.clone());
                    pc
                }
            };
            if let Some(pc) = prev_child {
                if pc != c {
                    constraint_graph.set_edge(pc, c, ());
                    break;
                }
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

    let mut down_layer_graphs = build_layer_graphs(graph, &down_ranks, Relationship::InEdges);
    let mut up_layer_graphs = build_layer_graphs(graph, &up_ranks, Relationship::OutEdges);

    let mut layering = init_order(graph);
    assign_order(graph, &layering);

    if opts.disable_optimal_order_heuristic {
        return;
    }

    let mut best_cc = f64::INFINITY;
    let mut best: Option<Vec<Vec<String>>> = None;

    let mut last_best = 0;
    let mut i = 0;
    while last_best < 4 {
        let lgs = if i % 2 == 1 {
            &mut down_layer_graphs
        } else {
            &mut up_layer_graphs
        };
        sweep_layer_graphs(lgs, i % 4 >= 2, graph);
        layering = util::build_layer_matrix(graph);
        let cc = cross_count(graph, &layering) as f64;
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
        i += 1;
    }

    if let Some(b) = best {
        assign_order(graph, &b);
    }
}

#[derive(Clone, Copy)]
enum Relationship {
    InEdges,
    OutEdges,
}

fn build_layer_graphs(
    graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
    ranks: &[i32],
    relationship: Relationship,
) -> Vec<LayerGraph> {
    let mut nodes_by_rank: HashMap<i32, Vec<String>> = HashMap::new();
    for v in graph.nodes() {
        if let Some(n) = graph.node(&v) {
            if let Some(r) = n.rank {
                nodes_by_rank.entry(r).or_default().push(v.clone());
            }
            if let (Some(min), Some(max)) = (n.min_rank, n.max_rank) {
                for r in min..=max {
                    if Some(r) != n.rank {
                        nodes_by_rank.entry(r).or_default().push(v.clone());
                    }
                }
            }
        }
    }
    ranks
        .iter()
        .map(|r| {
            build_layer_graph(
                graph,
                *r,
                relationship,
                nodes_by_rank.get(r).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

fn build_layer_graph(
    graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
    rank: i32,
    relationship: Relationship,
    nodes_with_rank: Vec<String>,
) -> LayerGraph {
    let root = create_root_node(graph);
    let mut result: LayerGraph = Graph::with_opts(GraphOpts {
        directed: true,
        multigraph: false,
        compound: true,
    });
    result.set_graph(LayerGraphLabel { root: root.clone() });
    // root node
    result.set_node(root.clone(), LayerNode::default());

    for v in &nodes_with_rank {
        let node = match graph.node(v) {
            Some(n) => n.clone(),
            None => continue,
        };
        let in_range = node.rank == Some(rank)
            || (node.min_rank.is_some()
                && node.max_rank.is_some()
                && node.min_rank.unwrap() <= rank
                && rank <= node.max_rank.unwrap());
        if !in_range {
            continue;
        }
        let parent = graph.parent(v).map(|s| s.to_string()).unwrap_or(root.clone());
        result.set_node(
            v.clone(),
            LayerNode {
                rank: node.rank,
                min_rank: node.min_rank,
                max_rank: node.max_rank,
                order: node.order,
                ..Default::default()
            },
        );
        result.set_parent(v, Some(&parent));

        let es = match relationship {
            Relationship::InEdges => graph.in_edges(v).unwrap_or_default(),
            Relationship::OutEdges => graph.out_edges(v).unwrap_or_default(),
        };
        for e in es {
            let u = if e.v == *v { e.w.clone() } else { e.v.clone() };
            // Make sure u exists in result before fetching edge.
            if !result.has_node(&u) {
                result.set_node(u.clone(), LayerNode::default());
                result.set_parent(&u, Some(&root));
            }
            let prev = result.edge(&u, v).map(|l| l.weight).unwrap_or(0.0);
            let w = graph.edge_obj(&e).map(|l| l.weight).unwrap_or(0.0);
            result.set_edge(u.clone(), v.clone(), LayerEdge { weight: prev + w });
        }

        if let (Some(bl), Some(br)) = (&node.border_left, &node.border_right) {
            let ri = rank as usize;
            if ri < bl.len() && ri < br.len() {
                if let Some(n) = result.node_mut(v) {
                    n.border_left = Some(bl[ri].clone());
                    n.border_right = Some(br[ri].clone());
                }
            }
        }
    }
    result
}

fn create_root_node(graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> String {
    loop {
        let v = util::unique_id("_root");
        if !graph.has_node(&v) {
            return v;
        }
    }
}

// ---------- init order ---------------------------------------------------

/// Public init-order — visible for testing. Performs a DFS starting at the
/// leftmost rank node, assigning visit order to each non-subgraph node.
pub fn init_order(graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>) -> Vec<Vec<String>> {
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let simple_nodes: Vec<String> = graph
        .nodes()
        .into_iter()
        .filter(|v| graph.children(Some(v)).is_empty())
        .collect();
    let max_rank = simple_nodes
        .iter()
        .filter_map(|v| graph.node(v).and_then(|n| n.rank))
        .max()
        .unwrap_or(0);
    let mut layers: Vec<Vec<String>> = (0..=max_rank.max(0)).map(|_| Vec::new()).collect();
    if max_rank < 0 {
        return Vec::new();
    }

    fn dfs(
        graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
        v: &str,
        visited: &mut std::collections::HashSet<String>,
        layers: &mut Vec<Vec<String>>,
    ) {
        if visited.contains(v) {
            return;
        }
        visited.insert(v.to_string());
        if let Some(n) = graph.node(v) {
            if let Some(r) = n.rank {
                if r >= 0 {
                    let ri = r as usize;
                    while layers.len() <= ri {
                        layers.push(Vec::new());
                    }
                    layers[ri].push(v.to_string());
                }
            }
        }
        let succ = graph.successors(v).unwrap_or_default();
        for s in succ {
            dfs(graph, &s, visited, layers);
        }
    }

    let mut ordered = simple_nodes.clone();
    ordered.sort_by_key(|v| graph.node(v).and_then(|n| n.rank).unwrap_or(0));
    for v in ordered {
        dfs(graph, &v, &mut visited, &mut layers);
    }
    layers
}

fn assign_order(graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>, layering: &[Vec<String>]) {
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
    layering: &[Vec<String>],
) -> u64 {
    let mut cc = 0u64;
    for i in 1..layering.len() {
        cc += two_layer_cross_count(graph, &layering[i - 1], &layering[i]);
    }
    cc
}

fn two_layer_cross_count(
    graph: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
    north: &[String],
    south: &[String],
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
    for v in north {
        let es = graph.out_edges(v).unwrap_or_default();
        let mut local: Vec<(usize, f64)> = Vec::new();
        for e in es {
            if let Some(&pos) = south_pos.get(e.w.as_str()) {
                let weight = graph.edge_obj(&e).map(|l| l.weight).unwrap_or(0.0);
                local.push((pos, weight));
            }
        }
        local.sort_by_key(|x| x.0);
        south_entries.extend(local);
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
        cc += weight * weight_sum;
    }
    cc as u64
}

// ---------- barycenter (internal LayerGraph version) ---------------------

fn barycenter_impl(graph: &LayerGraph, movable: &[String]) -> Vec<BarycenterEntry> {
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
                    let edge_w = graph.edge_obj(&e).map(|l| l.weight).unwrap_or(0.0);
                    let order = graph
                        .node(&e.v)
                        .and_then(|n| n.order)
                        .unwrap_or(0) as f64;
                    sum += edge_w * order;
                    weight += edge_w;
                }
                BarycenterEntry {
                    v: v.clone(),
                    barycenter: Some(if weight > 0.0 { sum / weight } else { 0.0 }),
                    weight: Some(weight),
                }
            }
        })
        .collect()
}

// ---------- resolve conflicts --------------------------------------------

fn resolve_conflicts_impl(
    entries: Vec<BarycenterEntry>,
    constraint_graph: &Graph<(), (), ()>,
) -> Vec<ResolvedEntry> {
    #[derive(Debug)]
    struct Mapped {
        indegree: usize,
        ins: Vec<usize>,
        outs: Vec<usize>,
        vs: Vec<String>,
        i: usize,
        barycenter: Option<f64>,
        weight: Option<f64>,
        merged: bool,
    }

    let mut mapped: Vec<Mapped> = Vec::with_capacity(entries.len());
    let mut v_to_idx: HashMap<String, usize> = HashMap::new();
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
        let (Some(&vi), Some(&wi)) = (v_to_idx.get(&e.v), v_to_idx.get(&e.w)) else {
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
        if let (Some(b), Some(w)) = (m[target].barycenter, m[target].weight) {
            if w != 0.0 {
                sum += b * w;
                weight += w;
            }
        }
        if let (Some(b), Some(w)) = (m[source].barycenter, m[source].weight) {
            if w != 0.0 {
                sum += b * w;
                weight += w;
            }
        }
        let source_vs = std::mem::take(&mut m[source].vs);
        let new_vs = {
            let mut v = source_vs;
            v.extend(std::mem::take(&mut m[target].vs));
            v
        };
        m[target].vs = new_vs;
        m[target].barycenter = if weight > 0.0 { Some(sum / weight) } else { None };
        m[target].weight = if weight > 0.0 { Some(weight) } else { None };
        m[target].i = m[target].i.min(m[source].i);
        m[source].merged = true;
    }

    let mut entries_out: Vec<usize> = Vec::new();
    while let Some(idx) = source_set.pop() {
        entries_out.push(idx);
        // Handle ins (reverse).
        let ins: Vec<usize> = mapped[idx].ins.iter().copied().rev().collect();
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
    let (sortable, mut unsortable) = util::partition(entries, |e| e.barycenter.is_some());
    unsortable.sort_by_key(|e| std::cmp::Reverse(e.i));
    let mut sortable = sortable;
    sortable.sort_by(|a, b| {
        let ab = a.barycenter.unwrap();
        let bb = b.barycenter.unwrap();
        if ab < bb {
            std::cmp::Ordering::Less
        } else if ab > bb {
            std::cmp::Ordering::Greater
        } else if !bias_right {
            a.i.cmp(&b.i)
        } else {
            b.i.cmp(&a.i)
        }
    });

    let mut vs: Vec<Vec<String>> = Vec::new();
    let mut sum = 0.0;
    let mut weight = 0.0;
    let mut vs_index = 0usize;

    let consume_unsortable =
        |vs: &mut Vec<Vec<String>>, unsortable: &mut Vec<ResolvedEntry>, mut index: usize| -> usize {
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
        sum += bc * w;
        weight += w;
        vs.push(entry.vs);
        vs_index = consume_unsortable(&mut vs, &mut unsortable, vs_index);
    }

    let flat: Vec<String> = vs.into_iter().flatten().collect();
    SortResult {
        vs: flat,
        barycenter: if weight > 0.0 { Some(sum / weight) } else { None },
        weight: if weight > 0.0 { Some(weight) } else { None },
    }
}

// ---------- sort subgraph ------------------------------------------------

fn sort_subgraph(
    graph: &LayerGraph,
    v: &str,
    constraint_graph: &mut Graph<(), (), ()>,
    bias_right: bool,
) -> SortResult {
    let children = graph.children(Some(v));
    let movable: Vec<String> = children;

    let barycenters = barycenter_impl(graph, &movable);
    let mut entries = barycenters;
    let subgraphs: HashMap<String, SortResult> = HashMap::new();
    // Layer graphs from build_layer_graph are flat (compound-but-rooted), no
    // nested subgraphs beyond the root level in our scope. So skip subgraph
    // recursion here. (Compound dagre input is out of scope per user choice.)
    let _ = subgraphs;

    let resolved = resolve_conflicts_impl(entries.drain(..).collect(), constraint_graph);
    sort_impl(resolved, bias_right)
}

// ---------- add subgraph constraints (no-op in our scope) -----------------

fn add_subgraph_constraints_layer(
    _layer: &LayerGraph,
    _cg: &mut Graph<(), (), ()>,
    _vs: &[String],
) {
    // No compound subgraphs to add constraints for in our scope.
}

// ---------- sweep --------------------------------------------------------

fn sweep_layer_graphs(
    layer_graphs: &mut Vec<LayerGraph>,
    bias_right: bool,
    graph: &mut Graph<GraphLabel, NodeLabel, EdgeLabel>,
) {
    let mut cg: Graph<(), (), ()> = Graph::new();
    for lg in layer_graphs.iter_mut() {
        let root = lg.graph().map(|g| g.root.clone()).unwrap_or_default();
        let sorted = sort_subgraph(&lg, &root, &mut cg, bias_right);
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
