use std::collections::HashMap;

use explorer_types::{IncomingModuleDeps, ModuleId};
use serde::{Deserialize, Serialize};

/// the name of the dep graph cache file, relative to the module root
pub(super) const CACHE_FILE_NAME: &str = "_cache.json";

/// the schema for `.modules/_cache.json`
#[derive(Serialize, Deserialize)]
pub struct CachedDepGraph {
	pub(super) version: String,
	/// what the module root looked like when this graph was built
	#[serde(default)]
	pub(super) fingerprint: ModuleRootFingerprint,
	pub(super) inverse_deps: HashMap<ModuleId, IncomingModuleDeps>,
}

/// A cheap summary of the module root, used to notice that `.modules` has been
/// re-dumped since a dep graph was cached.
#[derive(
	Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug,
)]
pub(super) struct ModuleRootFingerprint {
	/// how many `*.js` files the module root held
	modules: usize,
	/// the newest mtime across them, in milliseconds since the unix epoch
	newest_mtime_ms: u128,
}

impl ModuleRootFingerprint {
	/// Fold one module file's mtime into the fingerprint.
	pub(super) fn add_module(&mut self, mtime_ms: u128) {
		self.modules += 1;
		self.newest_mtime_ms = self.newest_mtime_ms.max(mtime_ms);
	}
}
