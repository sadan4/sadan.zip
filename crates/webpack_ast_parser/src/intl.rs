//! Resolution of hashed Discord i18n keys back to their original (unhashed)
//! message names.
//!
//! The mapping is embedded at compile time from the repo's
//! `src/utils/discordI18n/key-mappings.json`, which maps the 6-char hashed key
//! (e.g. `Go5Vvs`) to the original `SCREAMING_SNAKE_CASE` message name.

use std::{collections::HashMap, sync::LazyLock};

use smol_str::SmolStr;

static KEY_MAPPINGS_MPK_ZST: &[u8] = include_bytes!("./key_mappings.mpk.zst");

/// Parsed, lazily-initialised map of hashed key -> unhashed message name.
static KEY_MAPPINGS: LazyLock<HashMap<SmolStr, SmolStr>> =
	LazyLock::new(|| {
		let raw = zstd::decode_all(KEY_MAPPINGS_MPK_ZST)
			.expect("Failed to decompress key_mappings.mpk.zst");
		rmp_serde::from_slice(&raw)
			.expect("Failed to parse key_mappings.mpk.zst")
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
