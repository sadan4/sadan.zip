#![feature(trim_prefix_suffix)]
#![feature(path_trailing_sep)]
#![feature(try_blocks)]
#![feature(duration_constants)]
#![feature(current_thread_id)]
#![feature(integer_widen_truncate)]
#![allow(clippy::multiple_inherent_impl)]
pub mod lsp;
mod module_cache;
mod wss;
pub const SERVER_NAME: &str = "vencord-companion";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type JValue = serde_json::Value;
pub type LspResult<T> = tower_lsp_server::jsonrpc::Result<T>;
mod util;

pub struct State {
	ws: wss::WsServer,
}

pub type ReloadHandle = tracing_subscriber::reload::Handle<
	tracing_subscriber::EnvFilter,
	tracing_subscriber::Registry,
>;
