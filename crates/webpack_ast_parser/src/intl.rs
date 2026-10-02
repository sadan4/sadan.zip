pub mod ast;
use std::{collections::HashMap, sync::LazyLock};

use smol_str::SmolStr;

static KEY_MAPPINGS_MPK_ZST: &[u8] = include_bytes!("./key_mappings.mpk.zst");

/// Parsed, lazily-initialised map of hashed key -> unhashed message name.
static KEY_MAPPINGS: LazyLock<HashMap<SmolStr, SmolStr>> =
	LazyLock::new(|| {
		let raw = zstd::Decoder::new(KEY_MAPPINGS_MPK_ZST)
			.expect("Failed to decompress key_mappings.mpk.zst");
		rmp_serde::from_read(raw).expect("Failed to parse key_mappings.mpk.zst")
	});

/// Attempt to resolve a hashed i18n key to its original (unhashed) message
/// name. Returns `None` when the key is not present in the mapping.
pub fn resolve_unhashed_key(hashed: &str) -> Option<SmolStr> {
	KEY_MAPPINGS.get(hashed).cloned()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_key_mappings() {
		LazyLock::force(&KEY_MAPPINGS);
	}
}
