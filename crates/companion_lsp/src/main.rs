use companion_lsp::lsp;
use tokio::io;

#[tokio::main]
async fn main() {
	init_tracing();
	let srv = lsp::Server {};
	let (service, socket) = tower_lsp::LspService::new(|_| srv);
	tower_lsp::Server::new(io::stdin(), io::stdout(), socket)
		.serve(service)
		.await;
}

fn init_tracing() {
	use tracing_subscriber::{EnvFilter, fmt};

	let filter = EnvFilter::try_from_env("COMPANION_LSP_LOG")
		.unwrap_or_else(|_| EnvFilter::new("info"));

	let _ = fmt()
		.with_env_filter(filter)
		.with_writer(std::io::stderr)
		.with_ansi(false)
		.try_init();
}
