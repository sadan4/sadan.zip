use const_format::formatc;
use wasm_bindgen::prelude::wasm_bindgen;

// pub const SERVER_BASE_URL: &str = "https://s-d-br.sadan.zip";

const IS_SERVER_LOCAL: bool = cfg!(feature = "local-server");
#[cfg(not(debug_assertions))]
const _: () = {
	assert!(
		!IS_SERVER_LOCAL,
		"IS_SERVER_LOCAL must be false in release builds"
	);
};

pub(crate) const SERVER_BASE_URL: &str = if IS_SERVER_LOCAL {
	"http://localhost:8484"
} else {
	"https://s-d-br.sadan.zip"
};

pub(crate) const LIST_BUILDS_ENDPOINT: &str =
	formatc!("{SERVER_BASE_URL}/builds");

#[expect(non_snake_case)]
pub(crate) fn FULL_BUNDLE_ENDPOINT(build_hash: &str) -> String {
	format!("{SERVER_BASE_URL}/build/{build_hash}/full")
}

#[wasm_bindgen]
pub fn bundle_tarball_url(build_hash: &str) -> String {
	format!("{SERVER_BASE_URL}/build/{build_hash}/archive.tar.zst")
}

#[wasm_bindgen]
pub fn bundle_tarball_filename(build_hash: &str) -> String {
	format!("{build_hash}.tar.zst")
}
