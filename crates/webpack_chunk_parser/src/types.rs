#![allow(clippy::redundant_pub_crate)]
use derive_more::{From, Into};
use serde::Serialize;
use smol_str::SmolStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, PartialOrd, Ord, Serialize)]
pub struct ModuleId(pub u32);

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
