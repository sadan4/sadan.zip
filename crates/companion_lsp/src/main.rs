use companion_lsp::{ReloadHandle, lsp};
use tokio::io;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
	let reload_handle = init_tracing();
	let (service, socket) = LspService::new(|client| {
		lsp::Server::new(client).with_reload_handle(reload_handle)
	});
	Server::new(io::stdin(), io::stdout(), socket)
		.serve(service)
		.await;
}

fn init_tracing() -> ReloadHandle {
	use tracing_subscriber::{
		EnvFilter,
		fmt,
		layer::SubscriberExt as _,
		registry,
		reload,
		util::SubscriberInitExt as _,
	};

	let filter =
		EnvFilter::try_from_env("COMPANION_LSP_LOG").unwrap_or_else(|_| {
			if cfg!(debug_assertions) {
				EnvFilter::new("debug")
			} else {
				EnvFilter::new("info")
			}
		});

	let (reload_layer, reload_handle) = reload::Layer::new(filter);

	let fmt_layer = fmt::layer()
		.with_writer(std::io::stderr)
		.with_ansi(false);

	registry()
		.with(reload_layer)
		.with(fmt_layer)
		.init();
	reload_handle
}
