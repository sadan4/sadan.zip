pub mod cmds;
mod custom;

use std::sync::Arc;

use tower_lsp::{
	Client,
	LanguageServer,
	LspService,
	async_trait,
	jsonrpc::Result,
	lsp_types::{
		ExecuteCommandParams,
		InitializeParams,
		InitializeResult,
		ServerCapabilities,
		ServerInfo,
	},
};
use tracing::error;

use crate::{
	JValue,
	SERVER_NAME,
	SERVER_VERSION,
	State,
	wss::{self, WsServer},
};

pub struct Server {
	client: Client,
	state: Arc<State>,
}

impl Server {
	pub fn new(client: Client) -> Self {
		let state = Arc::new(State {
			ws: WsServer::disconnected(),
		});
		let server = state.ws.clone();
		tokio::spawn(async move {
			if let Err(e) = WsServer::run_loop(server).await {
				error!("WebSocket server failed: {e:?}");
				return;
			}
			error!("WebSocket server exited unexpectedly");
		});
		Self { client, state }
	}
}

#[async_trait]
impl LanguageServer for Server {
	async fn initialize(
		&self,
		params: InitializeParams,
	) -> Result<InitializeResult> {
		Ok(InitializeResult {
			capabilities: ServerCapabilities {
				execute_command_provider: Some(Self::get_cmd_provider()),
				..ServerCapabilities::default()
			},
			server_info: Some(ServerInfo {
				name: String::from(SERVER_NAME),
				version: Some(String::from(SERVER_VERSION)),
			}),
		})
	}

	async fn execute_command(
		&self,
		params: ExecuteCommandParams,
	) -> Result<Option<JValue>> {
		self.handle_cmd(params).await
	}

	async fn shutdown(&self) -> Result<()> {
		todo!()
	}
}
