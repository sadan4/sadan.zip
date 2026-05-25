use std::collections::HashMap;

use crate::Path;

pub struct ExtractedPath {
	pub weight: i32,
	pub path: Vec<String>,
}

pub fn extract_path(
	shortest_paths: &HashMap<String, Path>,
	source: &str,
	destination: &str,
) -> ExtractedPath {
	if !shortest_paths
		.get(source)
		.is_some_and(|a| a.predecessor.is_empty())
	{
		panic!("Invalid source vertex");
	}
	if shortest_paths
		.get(destination)
		.is_some_and(|a| a.predecessor.is_empty())
		&& destination != source
	{
		panic!("Invalid destination vertex");
	}
	ExtractedPath {
		weight: shortest_paths[destination].distance,
		path: run_extract_path(shortest_paths, source, destination),
	}
}

fn run_extract_path(
	shortest_paths: &HashMap<String, Path>,
	source: &str,
	destination: &str,
) -> Vec<String> {
	let mut path = Vec::new();
	let mut cur = destination;
	while cur != source {
		path.push(cur.to_string());
		cur = &shortest_paths[cur].predecessor;
	}
	path.push(source.to_string());
	path.reverse();
	path
}
