mod cmds;

use tower_lsp::{
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

use crate::{JValue, SERVER_NAME, SERVER_VERSION};

pub struct Server {}

#[async_trait]
impl LanguageServer for Server {
	async fn initialize(
		&self,
		params: InitializeParams,
	) -> Result<InitializeResult> {
		Ok(InitializeResult {
			capabilities: ServerCapabilities {
				execute_command_provider: Some(self.get_cmd_provider()),
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
