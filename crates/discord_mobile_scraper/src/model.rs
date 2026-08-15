use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
	pub metadata: Metadata,
	pub hashes: HashMap<String, String>,
	pub patches: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
	pub build: String,
	pub commit: String,
	pub confirm_update: bool,
	pub release_name: String,
}

#[cfg(test)]
mod tests {
	use super::*;
	pub fn de_m(s: &str) -> Manifest {
		serde_json::from_str(s).unwrap()
	}

	#[test]
	fn test_deserialize_manifest() {
		de_m(include_str!("./manifest.json"));
	}
}
