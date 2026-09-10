#![feature(trim_prefix_suffix)]
#![feature(try_blocks)]
#![allow(clippy::multiple_inherent_impl)]
pub mod lsp;
mod wss;
mod module_cache;
pub const SERVER_NAME: &str = "vencord-companion";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type JValue = serde_json::Value;
pub type LspResult<T> = tower_lsp::jsonrpc::Result<T>;

pub struct State {
	ws: wss::WsServer,
}

pub type ReloadHandle = tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, tracing_subscriber::Registry>;
