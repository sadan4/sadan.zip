use std::{
	fmt::{self, Display},
	io,
};

use wasm_bindgen::JsValue;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Failed to cast JsValue to {0}")]
	BadJsCast(#[from] BadCast),
	#[error("Protobuf deserialization: {0}")]
	ProtoDecode(#[from] prost::DecodeError),
	#[error("ZSTD: {0}")]
	Zstd(io::Error),
	#[error("HTTP request to {url} failed with code {status}")]
	BadRequest { status: u16, url: String },
	#[error("JS Error: {0:?}")]
	Js(JsValue),
	#[error("WASM Error: {0}")]
	Other(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum BadCast {
	ArrayBuffer,
	Response,
}

impl Display for BadCast {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:?}")
	}
}

impl From<JsValue> for Error {
	fn from(value: JsValue) -> Self {
		Self::Js(value)
	}
}

impl From<Error> for JsValue {
	fn from(val: Error) -> Self {
		js_sys::Error::new(&val.to_string()).into()
	}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
