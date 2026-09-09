use companion_lsp::lsp;
use tokio::io;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
	init_tracing();
	let (service, socket) = LspService::new(lsp::Server::new);
	Server::new(io::stdin(), io::stdout(), socket)
		.serve(service)
		.await;
}

fn init_tracing() {
	use tracing_subscriber::{EnvFilter, fmt};

	let filter = EnvFilter::try_from_env("COMPANION_LSP_LOG")
		.unwrap_or_else(|_| EnvFilter::new("info"));

	fmt()
		.with_env_filter(filter)
		.with_writer(std::io::stderr)
		.with_ansi(false)
		.try_init()
		.expect("Failed to init tracing");
}
