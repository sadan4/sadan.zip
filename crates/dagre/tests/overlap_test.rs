use std::collections::BTreeMap;

use dagre::{LayoutGraph, layout};

#[test]
fn panic1_no_overlapping_nodes() {
	type Row = (String, f64, f64, f64, f64, Option<i32>, Option<usize>);
	let json = include_str!("data/panic1.json");
	let mut g: LayoutGraph =
		serde_json::from_str(json).expect("deserialize panic1.json");
	layout(&mut g);

	let mut rows: Vec<Row> = Vec::new();
	for v in g.nodes() {
		if let Some(n) = g.node(&v) {
			if n.dummy.is_some() {
				continue;
			}
			if let (Some(x), Some(y)) = (n.x, n.y) {
				rows.push((
					v.clone(),
					x,
					y,
					n.width,
					n.height,
					n.rank,
					n.order,
				));
			}
		}
	}
	let mut by_y: BTreeMap<i64, Vec<&Row>> = BTreeMap::new();
	for r in &rows {
		by_y.entry(r.2.round() as i64)
			.or_default()
			.push(r);
	}
	let mut overlaps = 0;
	let mut samples = Vec::new();
	for (yk, mut row) in by_y {
		row.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
		for w in row.windows(2) {
			let a = w[0];
			let b = w[1];
			let a_right = a.1 + a.3 / 2.0;
			let b_left = b.1 - b.3 / 2.0;
			if a_right > b_left + 0.001 {
				overlaps += 1;
				if samples.len() < 10 {
					samples.push(format!(
						"y={yk} {} (x={:.1},w={:.1},rank={:?},order={:?}) overlaps {} (x={:.1},w={:.1},rank={:?},order={:?})",
						a.0, a.1, a.3, a.5, a.6, b.0, b.1, b.3, b.5, b.6
					));
				}
			}
		}
	}
	for s in &samples {
		eprintln!("{s}");
	}
	assert_eq!(
		overlaps, 0,
		"found {overlaps} overlapping pairs (showing up to 10)"
	);
}
