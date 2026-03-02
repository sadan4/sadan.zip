use std::io;

use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::util::console_log;

#[wasm_bindgen]
pub fn deserialize(buf: Box<[u8]>) -> Result<(), JsValue> {
    let mut tarball = Vec::new();
    console_log!("Compressed data length: {}", buf.len());
    zstd::stream::copy_decode(&*buf, &mut tarball)
        .map_err(|err| format!("Failed to decompress data: {err}"))?;
    console_log!("Decompressed data length: {}", tarball.len());

    let mut archive = tar::Archive::new(&*tarball);

    for entry in archive.entries().unwrap() {
        let mut file = entry.unwrap();
        console_log!("Extracting file: {:?}", file.path().unwrap());
    }

    console_log!("Extraction complete");
    Ok(())
}
