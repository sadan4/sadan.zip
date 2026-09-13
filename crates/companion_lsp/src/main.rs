use companion_lsp::{ReloadHandle, lsp};
use miette_ui::install_miette_hook;
use tokio::io;
use tower_lsp::{LspService, Server};

fn main() {
	setup_backtrace();
	tokio_main();
}

#[tokio::main]
async fn tokio_main() {
	install_miette_hook(false);
	let reload_handle = init_tracing();
	let builder = lsp::ServerBuilder::new()
		.expect("Failed to create LSP ServerBuilder")
		.with_reload_handle(reload_handle);
	let (service, socket) = LspService::new(|client| builder.build(client));
	Server::new(io::stdin(), io::stdout(), socket)
		.serve(service)
		.await;
}
fn setup_backtrace() {
	use std::env;
	if env::var_os("RUST_BACKTRACE").is_none() {
		// SAFETY: no other threads are running yet
		unsafe {
			env::set_var("RUST_BACKTRACE", "1");
		}
	}
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
		.with_ansi_sanitization(false)
		.with_ansi(false);

	registry()
		.with(reload_layer)
		.with(fmt_layer)
		.init();
	reload_handle
}
