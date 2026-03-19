#![allow(
    clippy::missing_const_for_fn,
    reason = "napi does not support const fns"
)]
use std::{collections::HashMap, io, mem};

use anyhow::{Context};
use explorer_server_core::{Channel as CoreChannel, EncodableBuild, write_full_bundle};
use explorer_types::{
    BundleMetadata, DepInfo, ExportName, FullBundle, KeyModules, ModuleDeps, TModuleId,
};
use napi_derive::napi;

#[napi]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Channel {
    Stable,
    Canary,
}

#[napi]
#[derive(Default, Debug, Clone)]
pub struct ProcessingMetadata {
    build_hash: String,
    build_number: u32,
    first_seen: i64,
    entry_point: Option<TModuleId>,
    env_var_text: String,
}

#[napi]
impl ProcessingMetadata {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    #[napi(setter)]
    pub fn set_build_hash(&mut self, build_hash: String) {
        self.build_hash = build_hash;
    }
    #[napi(setter)]
    pub fn set_build_number(&mut self, build_number: u32) {
        self.build_number = build_number;
    }
    #[napi]
    pub fn set_first_seen(&mut self, first_seen: i64) -> napi::Result<()> {
        if first_seen.is_negative() {
            return Err(napi::Error::from_reason(
                "first_seen must be a non-negative integer".to_string(),
            ));
        }
        self.first_seen = first_seen;
        Ok(())
    }
    #[napi(setter)]
    pub fn set_entry_point(&mut self, entry_point: Option<TModuleId>) {
        self.entry_point = entry_point;
    }
    #[napi(setter)]
    pub fn set_env_var_text(&mut self, env_var_text: String) {
        self.env_var_text = env_var_text;
    }
}

impl From<ProcessingMetadata> for BundleMetadata {
    fn from(
        ProcessingMetadata {
            build_hash,
            build_number,
            first_seen,
            entry_point,
            env_var_text,
        }: ProcessingMetadata,
    ) -> Self {
        Self {
            build_hash,
            build_number,
            first_seen: first_seen as u64,
            entry_point,
            env_var_text,
        }
    }
}

#[napi]
#[derive(Debug, Clone)]
pub enum ProcessingExportName {
    Named(String),
    Default,
}

impl From<ProcessingExportName> for ExportName {
    fn from(value: ProcessingExportName) -> Self {
        match value {
            ProcessingExportName::Named(name) => Self::Named(name),
            ProcessingExportName::Default => Self::Default,
        }
    }
}

#[napi]
#[derive(Default, Debug, Clone)]
pub struct ProcessingKeyModules {
    flux_dispatcher_class: Vec<(TModuleId, ProcessingExportName)>,
}

#[napi]
impl ProcessingKeyModules {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    #[napi]
    pub fn add_flux_dispatcher_class(
        &mut self,
        module_id: TModuleId,
        export_name: ProcessingExportName,
    ) {
        self.flux_dispatcher_class.push((module_id, export_name));
    }
}

impl From<ProcessingKeyModules> for KeyModules {
    fn from(value: ProcessingKeyModules) -> Self {
        Self {
            flux_dispatcher_class: value
                .flux_dispatcher_class
                .into_iter()
                .map(|(id, export_name)| (id, export_name.into()))
                .collect(),
        }
    }
}
#[napi]
#[derive(Default, Debug, Clone)]
pub struct ProcessingDepInfo {
    key_modules: ProcessingKeyModules,
    module_deps: HashMap<TModuleId, ModuleDeps>,
}

#[napi]
impl ProcessingDepInfo {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    #[napi(setter)]
    pub fn set_key_modules(&mut self, key_modules: &mut ProcessingKeyModules) {
        mem::swap(&mut self.key_modules, key_modules);
    }
    #[napi]
    pub fn add_sync_dep(&mut self, module_id: TModuleId, sync_use_id: TModuleId) {
        self.module_deps
            .entry(module_id)
            .or_default()
            .sync_uses
            .push(sync_use_id);
    }
    #[napi]
    pub fn add_lazy_dep(&mut self, module_id: TModuleId, lazy_use_id: TModuleId) {
        self.module_deps
            .entry(module_id)
            .or_default()
            .lazy_uses
            .push(lazy_use_id);
    }
}

impl From<ProcessingDepInfo> for DepInfo {
    fn from(
        ProcessingDepInfo {
            key_modules,
            module_deps,
        }: ProcessingDepInfo,
    ) -> Self {
        Self {
            key_modules: key_modules.into(),
            module_deps,
        }
    }
}

#[derive(Debug, Default, Clone)]
#[napi]
pub struct ProcessingBuild {
    metadata: ProcessingMetadata,
    dep_info: ProcessingDepInfo,
    module_sources: HashMap<String, Vec<TModuleId>>,
    modules: HashMap<TModuleId, String>,
}

#[napi]
impl ProcessingBuild {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    #[napi(setter)]
    pub fn set_metadata(&mut self, metadata: &mut ProcessingMetadata) {
        mem::swap(&mut self.metadata, metadata);
    }
    #[napi(setter)]
    pub fn set_dep_info(&mut self, dep_info: &mut ProcessingDepInfo) {
        mem::swap(&mut self.dep_info, dep_info);
    }
    #[napi]
    pub fn set_module_sources(&mut self, chunk_name: String, module_ids: Vec<TModuleId>) {
        self.module_sources.insert(chunk_name, module_ids);
    }
    #[napi]
    pub fn set_source(&mut self, module_id: TModuleId, source: String) {
        self.modules.insert(module_id, source);
    }
    #[napi]
    pub fn write(&mut self) -> napi::Result<()> {
        let bundle = mem::take(self).into();
        write_full_bundle(&bundle).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(())
    }
}

impl From<ProcessingBuild> for FullBundle {
    fn from(
        ProcessingBuild {
            metadata,
            dep_info,
            module_sources,
            modules,
        }: ProcessingBuild,
    ) -> Self {
        Self {
            metadata: metadata.into(),
            dep_info: dep_info.into(),
            module_sources,
            modules,
        }
    }
}

#[napi(object)]
pub struct WatcherInfo {
    pub build_hash: String,
    pub channel: Channel,
    pub web_js_url: String,
    pub global_env_text: String,
}

impl From<EncodableBuild> for WatcherInfo {
    fn from(
        EncodableBuild {
            channel,
            build_hash,
            web_js_url,
            global_env_text,
        }: EncodableBuild,
    ) -> Self {
        Self {
            build_hash,
            channel: match channel {
                CoreChannel::Canary => Channel::Canary,
                CoreChannel::Stable => Channel::Stable,
            },
            web_js_url,
            global_env_text,
        }
    }
}

#[napi]
pub fn read_stdin_data() -> napi::Result<WatcherInfo> {
    rmp_serde::from_read::<_, EncodableBuild>(io::stdin())
        .context("Failed to parse watcher data as MPK")
        .map(From::from)
        .map_err(From::from)
}

#[napi_derive::module_init]
fn init() {
    tracing_subscriber::fmt().init();
}
