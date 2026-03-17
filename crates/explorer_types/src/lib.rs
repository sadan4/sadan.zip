use serde::{Deserialize, Serialize};
use std::{collections::HashMap};

pub type TModuleId = u32;

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BundleMetadata {
    pub build_hash: String,
    pub build_number: u32,
    pub first_seen: u64,
    pub entry_point: Option<TModuleId>,
    pub env_var_text: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ExportName {
    Named(String),
    Default,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyModules {
    pub flux_dispatcher_class: Vec<(TModuleId, ExportName)>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDeps {
    pub sync_uses: Vec<TModuleId>,
    pub lazy_uses: Vec<TModuleId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DepInfo {
    pub key_modules: KeyModules,
    pub module_deps: HashMap<TModuleId, ModuleDeps>,
}

pub type ModuleSources = HashMap<String, Vec<TModuleId>>;

pub type Modules = HashMap<TModuleId, String>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FullBundle {
    pub metadata: BundleMetadata,
    pub dep_info: DepInfo,
    pub module_sources: HashMap<String, Vec<TModuleId>>,
    pub modules: HashMap<TModuleId, String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildList {
    /// array of zstd compressed msgpack serialized [`BundleMetadata`]
    pub builds: Vec<Box<[u8]>>,
}
