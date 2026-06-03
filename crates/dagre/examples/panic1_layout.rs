use std::hint::black_box;

use dagre::{LayoutGraph, layout};

fn main() {
	#[cfg(not(feature = "profile"))]
	compile_error!("useless without profile feature");
	let json = include_str!("../tests/data/panic1.json");
	let mut g: LayoutGraph = serde_json::from_str::<LayoutGraph>(json)
		.expect("failed to deserialize panic1.json");
	layout(&mut g);
	black_box(&mut g);
	println!("layout complete");
}
