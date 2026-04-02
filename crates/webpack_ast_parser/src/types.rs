use derive_more::{Deref, From, Into};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Deref, PartialOrd, Ord, Serialize)]
pub struct ModuleId(pub u32);
