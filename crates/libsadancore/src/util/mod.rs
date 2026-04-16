pub mod fut;
use crate::{
	err::{BadCast, Error, Result},
	util::fut::JsPromiseExt,
};
use js_sys::{ArrayBuffer, Uint8Array};
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Response, Window, window};

pub fn get_window() -> Window {
	window().unwrap()
}

// #[wasm_bindgen]
// extern "C" {
//     #[wasm_bindgen(js_namespace = console)]
//     pub(crate) fn log(s: &str);
// }

// macro_rules! console_log {
//     ($($t:tt)*) => ($crate::util::log(&format_args!($($t)*).to_string()));
// }

// pub(crate) use console_log;

pub async fn fetch(url: &str) -> Result<Response> {
	let w = get_window();
	let maybe_rsp = JsFuture::from(w.fetch_with_str(url)).await?;

	assert!(maybe_rsp.is_instance_of::<Response>());
	let rsp = maybe_rsp
		.dyn_into::<Response>()
		.map_err(|_| BadCast::Response)?;
	if !rsp.ok() {
		return Err(Error::BadRequest {
			status: rsp.status(),
			url: url.to_string(),
		});
	}
	Ok(rsp)
}

pub async fn fetch_struct<T>(url: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	let arr_buf = fetch(url)
		.await?
		.array_buffer()?
		.fut()
		.await?
		.dyn_into::<ArrayBuffer>()
		.map_err(|_| BadCast::ArrayBuffer)?;
	let zstd_raw_data = Uint8Array::new(&arr_buf).to_vec();
	let mpk_raw_data =
		zstd::decode_all(&*zstd_raw_data).map_err(Error::Zstd)?;
	let data = rmp_serde::from_slice(&mpk_raw_data)?;
	Ok(data)
}
