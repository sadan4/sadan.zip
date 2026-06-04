use derive_more::{Deref, Display, From, Into};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TModuleId = u32;

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BundleMetadata {
	pub build_hash: String,
	pub build_number: u32,
	pub first_seen: u64,
	pub entry_point: Option<ModuleId>,
	pub env_var_text: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ExportName {
	Default,
	#[serde(untagged)]
	Named(String),
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyModules {
	pub flux_dispatcher_class: Vec<(ModuleId, ExportName)>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DepInfo {
	pub key_modules: KeyModules,
	pub module_deps: HashMap<ModuleId, IncomingModuleDeps>,
}

pub type ModuleSources = HashMap<String, Vec<ModuleId>>;

pub type Modules = HashMap<ModuleId, String>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FullBundle {
	pub metadata: BundleMetadata,
	pub dep_info: DepInfo,
	pub module_sources: HashMap<String, Vec<ModuleId>>,
	pub modules: HashMap<ModuleId, String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildList {
	/// array of zstd compressed msgpack serialized [`BundleMetadata`]
	pub builds: Vec<Box<[u8]>>,
}

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	From,
	Into,
	Deref,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
	Display,
)]
pub struct ModuleId(pub u32);

impl TryFrom<f64> for ModuleId {
	// TODO: is this a good error type
	type Error = ();

	fn try_from(value: f64) -> Result<Self, Self::Error> {
		if value.fract() == 0. && value >= 0. && value <= f64::from(u32::MAX) {
			Ok(Self(value as u32))
		} else {
			Err(())
		}
	}
}

/// Information about a module's dependents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingModuleDeps {
	/// The modules that require this module synchronously
	pub sync: Vec<ModuleId>,
	/// the module that require this module lazily (dynamic import)
	pub lazy: Vec<ModuleId>,
}

/// Information about a module's dependencies
#[derive(Default, Clone, Debug)]
pub struct OutgoingModuleDeps {
	/// The modules that this module requires synchronously
	pub sync: Vec<ModuleId>,
	/// the module that this module requires lazily (dynamic import)
	pub lazy: Vec<ModuleId>,
}

impl OutgoingModuleDeps {
	pub const fn new() -> Self {
		Self {
			sync: Vec::new(),
			lazy: Vec::new(),
		}
	}
}

impl IncomingModuleDeps {
	pub const fn new() -> Self {
		Self {
			sync: Vec::new(),
			lazy: Vec::new(),
		}
	}
}
