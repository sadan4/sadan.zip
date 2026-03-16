use crate::{
    constants::LIST_BUILDS_ENDPOINT,
    err::Result,
    util::{fetch, fut::JsPromiseExt},
};
use explorer_types::{BuildList, BundleMetadata, FullBundle, TModuleId};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast as _, JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
pub struct Bundle {
    d: FullBundle,
}

#[wasm_bindgen]
pub struct Meta {
    d: BundleMetadata,
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

#[wasm_bindgen]
pub async fn get_builds() -> Result<Box<[Meta]>> {
    // fetch list of builds from the server
    // TODO: streaming decode
    let arr_buf = fetch(LIST_BUILDS_ENDPOINT)
        .await?
        .array_buffer()?
        .fut()
        .await?
        .dyn_into::<ArrayBuffer>()?;
    // this request is NOT zstd compressed as of now, it is just raw messagepack data
    let mpk_data_buf = Uint8Array::new(&arr_buf).to_vec();
    let data: BuildList = rmp_serde::from_slice(&mpk_data_buf)?;
    data.builds
        .into_iter()
        .map(|zstd_raw_meta| -> Result<_> {
            let mpk_raw_meta = zstd::decode_all(&*zstd_raw_meta)?;
            let d = rmp_serde::from_slice(&mpk_raw_meta)?;
            Ok(Meta { d })
        })
        .collect()
}

// zstd mpk data
// impl TryFrom<Box<[u8]>> for Meta {
// }
