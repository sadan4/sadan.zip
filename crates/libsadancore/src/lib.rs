use wasm_bindgen::prelude::wasm_bindgen;

pub mod explorer;
pub(crate) mod util;

#[wasm_bindgen(start)]
fn _start() {
    console_error_panic_hook::set_once();
}
