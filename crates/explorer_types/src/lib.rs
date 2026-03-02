#[cfg(feature = "napi")]
use napi::bindgen_prelude::FromNapiValue;
use napi::{
    Env, JsValue, Status, Unknown,
    bindgen_prelude::{Function, JsObjectValue, Object},
};
#[cfg(feature = "napi")]
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};

#[cfg(feature = "napi")]
mod napi_util {
    use super::{
        Env, FromNapiValue, Function, Hash, HashMap, JsObjectValue, JsValue, Object, Status,
        Unknown, napi,
    };
    #[napi(object)]
    struct JsIteratorValue<'env> {
        pub value: Option<Unknown<'env>>,
        pub done: bool,
    }

    #[napi(object)]
    struct JsIterator<'env> {
        pub next: Function<'env, (), JsIteratorValue<'env>>,
    }

    impl<'env> Iterator for JsIterator<'env> {
        type Item = napi::Result<Unknown<'env>>;

        fn next(&mut self) -> Option<Self::Item> {
            match self.next.call(()) {
                Ok(value) => {
                    if value.done {
                        None
                    } else {
                        Some(Ok(value.value.unwrap()))
                    }
                }
                Err(e) => Some(Err(e)),
            }
        }
    }

    pub fn from_js_map<K: FromNapiValue + Eq + Hash, V: FromNapiValue>(
        env: Env,
        map: Unknown,
    ) -> napi::Result<HashMap<K, V>> {
        let global_this = env.get_global()?;
        let global_map_ctor = global_this.get_named_property::<Object>("Map")?;
        let map = Object::from_unknown(map)?;
        if !map.instanceof(global_map_ctor)? {
            return Err(napi::Error::new(
                Status::InvalidArg,
                "expected a Map object",
            ));
        }
        let global_map_proto: Object = global_map_ctor.get_named_property("prototype")?;
        let global_map_proto_entries: Function<(), JsIterator> =
            global_map_proto.get_named_property("entries")?;
        let iter = global_map_proto_entries.apply(map, ())?;
        let mut ret = HashMap::new();

        for entry in iter {
            let entry = <(K, V)>::from_unknown(entry?)?;
            ret.insert(entry.0, entry.1);
        }

        Ok(ret)
    }
}

#[cfg(feature = "napi")]
use napi_util::from_js_map;

#[cfg_attr(feature = "napi", napi)]
pub type TModuleId = u32;

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Serialize, Deserialize)]
pub struct BundleMetadata {
    pub build_hash: String,
    pub build_number: u32,
    /// this is an u64 timestamp, but napi-rs doesn't support u64, only i64
    pub first_seen: i64,
    pub entry_point: Option<TModuleId>,
    pub env_var_text: String,
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Serialize, Deserialize)]
pub enum ExportName {
    Named(String),
    Default,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Serialize, Deserialize)]
pub struct KeyModules {
    pub flux_dispatcher_class: Vec<(TModuleId, ExportName)>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Serialize, Deserialize)]
pub struct ModuleDeps {
    pub sync_uses: Vec<TModuleId>,
    pub lazy_uses: Vec<TModuleId>,
}

#[derive(Serialize, Deserialize)]
pub struct DepInfo {
    pub key_modules: KeyModules,
    pub module_deps: HashMap<TModuleId, ModuleDeps>,
}

#[cfg(feature = "napi")]
impl FromNapiValue for DepInfo {
    unsafe fn from_napi_value(
        env_val: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let maybe_obj = unsafe { Unknown::from_napi_value(env_val, napi_val) }?;
        let obj = Object::from_unknown(maybe_obj)?;
        let key_modules = obj.get_named_property("keyModules")?;
        let module_deps = from_js_map(Env::from(env_val), obj.get_named_property("moduleDeps")?)?;
        Ok(Self {
            key_modules,
            module_deps,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct FullBundle {
    pub metadata: BundleMetadata,
    pub dep_info: DepInfo,
    pub module_sources: HashMap<String, Vec<TModuleId>>,
    pub modules: HashMap<TModuleId, String>,
}

#[cfg(feature = "napi")]
impl FromNapiValue for FullBundle {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let unknown = unsafe { Unknown::from_napi_value(env, napi_val) }?;
        let obj = Object::from_unknown(unknown)?;
        let metadata = obj.get_named_property("metadata")?;
        let dep_info = obj.get_named_property_unchecked("depInfo")?;
        let env = Env::from(env);
        let module_sources = obj.get_named_property("moduleSources")?;
        let modules = obj.get_named_property("modules")?;
        Ok(Self {
            metadata,
            dep_info,
            module_sources: from_js_map(env, module_sources)?,
            modules: from_js_map(env, modules)?,
        })
    }
}
