#![allow(clippy::redundant_pub_crate)]
use derive_more::{From, Into};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Serialize)]
pub struct ModuleId(pub u32);

#[derive(Debug, Clone, Into)]
pub(crate) struct ModuleEntry(pub(crate) ModuleId, pub(crate) String);

impl From<ModuleEntry> for (u32, String) {
	fn from(value: ModuleEntry) -> Self {
		(value.0.0, value.1)
	}
}
