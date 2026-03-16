use const_format::formatc;

// pub const SERVER_BASE_URL: &str = "https://s-d-br.sadan.zip";

const IS_SERVER_LOCAL: bool = true;

pub const SERVER_BASE_URL: &str = if IS_SERVER_LOCAL {
    "http://localhost:8484"
} else {
    "https://s-d-br.sadan.zip"
};

pub const LIST_BUILDS_ENDPOINT: &str = formatc!("{SERVER_BASE_URL}/builds");

#[expect(non_snake_case)]
pub fn FULL_BUNDLE_ENDPOINT(build_hash: &str) -> String {
    format!("{SERVER_BASE_URL}/bundle/{build_hash}/full")
}