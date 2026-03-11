use std::io;

use explorer_types::{BundleMetadata, FullBundle, TModuleId};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::util::console_log;

#[wasm_bindgen]
pub struct Bundle {
    d: FullBundle
}

#[wasm_bindgen]
pub struct Meta {
    d: BundleMetadata
}

#[wasm_bindgen]
pub struct BundleList {
    bundles: Vec<Meta>
}

#[wasm_bindgen]
impl Meta {
    #[wasm_bindgen(getter)]
    pub fn build_hash(&self) -> String {
        self.d.build_hash.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn build_number(&self) -> u32 {
        self.d.build_number
    }
    #[wasm_bindgen(getter)]
    pub fn first_seen(&self) -> u64 {
        self.d.first_seen
    }
    #[wasm_bindgen(getter)]
    pub fn entry_point(&self) -> Option<TModuleId> {
        self.d.entry_point
    }
    #[wasm_bindgen(getter)]
    pub fn env_var_text(&self) -> String {
        self.d.env_var_text.clone()
    }
}

// zstd mpk data
// impl TryFrom<Box<[u8]>> for Meta {
// }