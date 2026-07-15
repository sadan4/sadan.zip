//! Resolution of hashed Discord i18n keys back to their original (unhashed)
//! message names.
//!
//! The mapping is embedded at compile time from the repo's
//! `src/utils/discordI18n/key-mappings.json`, which maps the 6-char hashed key
//! (e.g. `Go5Vvs`) to the original `SCREAMING_SNAKE_CASE` message name.

use std::{collections::HashMap, sync::LazyLock};

use smol_str::SmolStr;

/// Raw JSON map of `hashedKey -> UNHASHED_NAME`, embedded from the repo.
static KEY_MAPPINGS_JSON: &str =
	include_str!("../../../src/utils/discordI18n/key-mappings.json");

/// Parsed, lazily-initialised map of hashed key -> unhashed message name.
///
/// Both keys and values borrow directly from the embedded, `'static` JSON
/// (zero-copy) — the mapping contains no escape sequences.
static KEY_MAPPINGS: LazyLock<HashMap<&'static str, &'static str>> =
	LazyLock::new(|| {
		serde_json::from_str(KEY_MAPPINGS_JSON)
			.expect("key-mappings.json is valid JSON of string -> string")
	});

/// Attempt to resolve a hashed i18n key to its original (unhashed) message
/// name. Returns `None` when the key is not present in the mapping.
pub fn resolve_unhashed_key(hashed: &str) -> Option<SmolStr> {
	KEY_MAPPINGS
		.get(hashed)
		.copied()
		.map(SmolStr::new_static)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_key_mappings() {
		LazyLock::force(&KEY_MAPPINGS);
	}
}
