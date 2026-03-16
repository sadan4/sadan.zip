use std::io;

use wasm_bindgen::JsValue;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("MPK deserialization: {0}")]
    MpkDecodeError(#[from] rmp_serde::decode::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("JS Error: {0:?}")]
    Js(JsValue),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Self::Js(value)
    }
}

impl From<Error> for JsValue {
    fn from(val: Error) -> Self {
        Self::from_str(&val.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
