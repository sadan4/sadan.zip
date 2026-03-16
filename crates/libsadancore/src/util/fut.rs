use js_sys::Promise;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

pub(crate) trait JsPromiseExt<T> {
    fn fut(self) -> JsFuture<T>;
}

impl JsPromiseExt<JsValue> for Promise {
    fn fut(self) -> JsFuture<JsValue> {
        JsFuture::from(self)
    }
}
