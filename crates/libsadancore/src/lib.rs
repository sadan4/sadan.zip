use wasm_bindgen::prelude::wasm_bindgen;

pub(crate) mod constants;
pub(crate) mod err;
pub mod explorer;
pub(crate) mod util;

#[wasm_bindgen(start)]
fn _start() {
	console_error_panic_hook::set_once();
}
