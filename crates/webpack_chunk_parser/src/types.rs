#![allow(clippy::redundant_pub_crate)]
use derive_more::Into;
use explorer_types::ModuleId;
use serde::Serialize;
use smol_str::SmolStr;

#[derive(Debug, Clone, Into)]
pub(crate) struct ModuleEntry(pub(crate) ModuleId, pub(crate) String);

impl From<ModuleEntry> for (u32, String) {
	fn from(value: ModuleEntry) -> Self {
		(value.0.0, value.1)
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct JsHashEntry {
	// at most 8 chars `/"\d{,6}"/`
	pub chunk_id: SmolStr,
	// 16 chars
	pub hash: SmolStr,
}
