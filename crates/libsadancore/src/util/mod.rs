pub mod fut;
use crate::Result;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{
    JsCast, prelude::{Closure, wasm_bindgen},
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, Response, Window, window};

pub fn get_window() -> Window {
    window().unwrap()
}

type AnimationFrameHandle = i32;

type RAFCallback = Closure<dyn FnMut()>;

pub fn request_animation_frame(f: &Rc<RefCell<Option<RAFCallback>>>) -> AnimationFrameHandle {
    get_window()
        .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .unwrap()
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;

    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

pub fn rand_32() -> f32 {
    random() as f32
}

pub fn rand_f32(min: f32, max: f32) -> f32 {
    debug_assert!(min <= max);
    rand_32() * (max - min) + min
}

macro_rules! console_log {
    ($($t:tt)*) => ($crate::util::log(&format_args!($($t)*).to_string()));
}

pub(crate) use console_log;

pub async fn fetch(url: &str) -> Result<Response> {
    let req = Request::new_with_str(url)?;
    let w = get_window();
    let maybe_rsp = JsFuture::from(w.fetch_with_str(url)).await?;

    assert!(maybe_rsp.is_instance_of::<Response>());
    let rsp = maybe_rsp.dyn_into::<Response>().unwrap();
    Ok(rsp)
}
