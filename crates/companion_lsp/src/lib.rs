pub mod lsp;
pub const SERVER_NAME: &str = "vc-companion-lsp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type JValue = serde_json::Value;
