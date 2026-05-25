use std::collections::HashSet;

use derive_more::IsVariant;

use crate::Graph;

#[derive(Copy, Clone, Debug, PartialEq, Eq, IsVariant)]
pub enum Order {
	Pre,
	Post,
}

pub fn reduce<T, GraphLabel, NodeLabel, EdgeLabel>(
	g: &Graph<GraphLabel, NodeLabel, EdgeLabel>,
	vs: Vec<String>,
	order: Order,
	func: impl Fn(T, String) -> T,
	mut acc: T,
) -> T {
	let navigation = |v: &str| {
		let ret = if g.is_directed() {
			g.successors(v.to_string())
		} else {
			g.neighbors(v.to_string())
		};
		ret.unwrap_or_default()
	};
	let mut visited = HashSet::new();
	for v in vs {
		if !g.has_node(v.clone()) {
			panic!("Graph does not have node: {v}");
		}
		acc = do_reduce(v, order, &mut visited, navigation, &func, acc);
	}

	acc
}

fn do_reduce<T>(
	v: String,
	order: Order,
	visited: &mut HashSet<String>,
	navigation: impl Fn(&str) -> Vec<String>,
	func: impl Fn(T, String) -> T,
	mut acc: T,
) -> T {
	if visited.contains(&v) {
		return acc;
	}
	visited.insert(v.clone());
	if !order.is_post() {
		acc = func(acc, v.clone());
	}
	for w in navigation(&v) {
		acc = do_reduce(w, order, visited, &navigation, &func, acc);
	}
	if order.is_post() {
		acc = func(acc, v);
	}
	acc
}
