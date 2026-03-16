use wasm_bindgen::prelude::wasm_bindgen;

pub(crate) mod err;
pub(crate) mod util;
pub(crate) mod constants;
pub mod explorer;

#[wasm_bindgen(start)]
fn _start() {
    console_error_panic_hook::set_once();
}
