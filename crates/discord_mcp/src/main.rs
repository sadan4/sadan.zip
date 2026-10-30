mod css;
mod progress;
mod server;
mod source;
mod store;
mod text;
mod tools;

use std::env;

use anyhow::Result;
use bundle_source::explorer::{DEFAULT_BASE_URL, ExplorerClient};
use miette::{GraphicalReportHandler, GraphicalTheme};
use rmcp::{ServiceExt as _, transport::stdio};
use tracing::info;
use tracing_subscriber::EnvFilter;

const LOG_ENV: &str = "DISCORD_MCP_LOG";
/// Overrides the explorer server, eg `http://localhost:8484`
const EXPLORER_URL_ENV: &str = "DISCORD_MCP_EXPLORER_URL";

#[tokio::main]
async fn main() -> Result<()> {
	// stdout carries the protocol, so logs must go to stderr
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_env(LOG_ENV)
				.unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.with_writer(std::io::stderr)
		.with_ansi(false)
		.init();
	// diagnostics are read by a model, not a terminal
	miette::set_hook(Box::new(|_| {
		Box::new(
			GraphicalReportHandler::new_themed(
				GraphicalTheme::unicode_nocolor(),
			)
			.without_syntax_highlighting()
			.with_context_lines(2),
		)
	}))?;

	let explorer_url =
		env::var(EXPLORER_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into());
	info!("Using explorer server {explorer_url}");
	let server = server::DiscordMcp::new(ExplorerClient::new(explorer_url))
		.serve(stdio())
		.await?;
	server.waiting().await?;
	Ok(())
}
