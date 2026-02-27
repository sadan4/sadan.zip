use wasm_bindgen::JsValue;
use web_sys::{CssStyleDeclaration, Document, Element, Window, window};

pub fn get_window() -> Window {
    window().unwrap()
}

pub fn get_document() -> Document {
    get_window().document().unwrap()
}

pub fn get_computed_style(el: &Element) -> Result<Option<CssStyleDeclaration>, JsValue> {
    get_window().get_computed_style(el)
}
