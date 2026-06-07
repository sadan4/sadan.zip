use std::sync::Arc;

use companion_lsp::{
	Backend,
	SessionState,
	discord_bridge,
	vencord_ext,
};
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
	init_tracing();

	let session = Arc::new(SessionState::new());

	let discord_session = session.clone();
	tokio::spawn(async move {
		if let Err(err) = discord_bridge::run(discord_session).await {
			tracing::error!(?err, "discord bridge terminated");
		}
	});

	let (service, socket) =
		LspService::build(|client| Backend::new(client, session.clone()))
			.custom_method(
				vencord_ext::QUICK_PICK_RESPONSE_METHOD,
				Backend::on_quick_pick_response,
			)
			.finish();

	let stdin = tokio::io::stdin();
	let stdout = tokio::io::stdout();
	Server::new(stdin, stdout, socket).serve(service).await;
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
