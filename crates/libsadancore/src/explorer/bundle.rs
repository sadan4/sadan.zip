
use crate::{
    constants::FULL_BUNDLE_ENDPOINT, err::Result, explorer::meta::Meta, util::fetch_struct,
};
use explorer_types::{DepInfo, FullBundle, ModuleSources, Modules, TModuleId};
use serde::Serialize;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
pub struct Bundle {
    metadata: Meta,
    dep_info: DepInfo,
    module_sources: ModuleSources,
    modules: Modules,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleDepsJs<'a> {
    sync_uses: &'a Vec<TModuleId>,
    lazy_uses: &'a Vec<TModuleId>,
}

#[wasm_bindgen]
impl Bundle {
    pub fn get_module_text(&self, module_id: TModuleId) -> Option<String> {
        self.modules.get(&module_id).cloned()
    }
    #[wasm_bindgen(skip_typescript)]
    pub fn get_module_deps(&self, module_id: TModuleId) -> Option<JsValue> {
        let deps = self.dep_info.module_deps.get(&module_id)?;
        let tmp = ModuleDepsJs {
            sync_uses: &deps.sync_uses,
            lazy_uses: &deps.lazy_uses,
        };
        let ret = serde_wasm_bindgen::to_value(&tmp).unwrap();
        Some(ret)
    }
    pub fn get_id_list(&self) -> Box<[TModuleId]> {
        let mut ret: Vec<_> = self.modules.keys().copied().collect();
        ret.sort_unstable();
        ret.into()
    }
    pub fn has_id(&self, module_id: TModuleId) -> bool {
        self.modules.contains_key(&module_id)
    }
}

#[wasm_bindgen(typescript_custom_section)]
const MODULE_DEPS_JS_TYPES: &str = r#"
    export interface Bundle {
        get_module_deps(module_id: number): {
            syncUses: number[];
            lazyUses: number[];
        } | undefined;
    }
"#;

#[wasm_bindgen]
pub async fn get_bundle(build_hash: &str, drop_sources: bool) -> Result<Bundle> {
    let FullBundle {
        metadata,
        dep_info,
        module_sources,
        modules,
    }: FullBundle = fetch_struct(&FULL_BUNDLE_ENDPOINT(build_hash)).await?;
    let module_sources = if drop_sources {
        ModuleSources::default()
    } else {
        module_sources
    };
    Ok(Bundle {
        metadata: metadata.into(),
        dep_info,
        module_sources,
        modules,
    })
}
