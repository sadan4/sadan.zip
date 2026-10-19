use std::collections::HashMap;

use serde::{Serialize, Serializer};

/// Serializes a [`HashMap`] as a map with its entries sorted by key.
///
/// [`HashMap`] iteration order is random, so two serializations of the same
/// bundle would otherwise produce different bytes. Stable ordering keeps
/// writes deterministic and lets deltas between builds find long matches.
pub fn serialize_sorted_map<K, V, S>(
	map: &HashMap<K, V>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	K: Serialize + Ord,
	V: Serialize,
	S: Serializer,
{
	let mut entries: Vec<_> = map.iter().collect();
	entries.sort_unstable_by_key(|&(k, _)| k);
	serializer.collect_map(entries)
}
