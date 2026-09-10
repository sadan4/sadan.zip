pub mod cmds;
mod custom;
mod definition;
mod doc;

use std::{debug_assert_matches, sync::Arc};

use tower_lsp::{
	Client, LanguageServer, async_trait, jsonrpc::Result, lsp_types::{
		DefinitionOptions, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, InitializeResult, OneOf, SaveOptions, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions, WorkDoneProgressOptions, notification::DidOpenTextDocument,
	},
};
use tracing::{error, info, instrument};
use tracing_subscriber::reload;

use crate::{
	JValue, LspResult, ReloadHandle, SERVER_NAME, SERVER_VERSION, State, wss::WsServer,
};

pub struct Server {
	client: Client,
	files: doc::Files,
	state: Arc<State>,
	log_reload_handle: Option<ReloadHandle>,
}

impl Server {
	#[must_use]
	pub fn new(client: Client) -> Self {
		let state = Arc::new(State {
			ws: WsServer::disconnected(),
		});
		let server = state.ws.clone();
		tokio::spawn(async move {
			if let Err(e) = server.run_loop().await {
				error!("WebSocket server failed: {e:?}");
				return;
			}
			error!("WebSocket server exited unexpectedly");
		});
		let files = doc::Files::default();
		Self {
			client,
			files,
			state,
			log_reload_handle: None,
		}
	}

	#[must_use]
	pub fn with_reload_handle(mut self, handle: ReloadHandle) -> Self {
		debug_assert_matches!(self.log_reload_handle, None, "reload handle already set");
		self.log_reload_handle = Some(handle);
		self
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
				text_document_sync: Some(TextDocumentSyncCapability::Options(
					TextDocumentSyncOptions {
						open_close: Some(true),
						change: Some(TextDocumentSyncKind::INCREMENTAL),
						will_save: Some(true),
						save: Some(TextDocumentSyncSaveOptions::SaveOptions(
							SaveOptions {
								include_text: Some(true),
							},
						)),
						..Default::default()
					},
				)),
				definition_provider: Some(OneOf::Left(true)),
				execute_command_provider: Some(Self::get_cmd_provider()),
				..ServerCapabilities::default()
			},
			server_info: Some(ServerInfo {
				name: String::from(SERVER_NAME),
				version: Some(String::from(SERVER_VERSION)),
			}),
		})
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri))]
	async fn did_open(&self, params: DidOpenTextDocumentParams) {
		self.files.handle_open(params);
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri))]
	async fn did_change(&self, params: DidChangeTextDocumentParams) {
		self.files.handle_change(params);
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri))]
	async fn did_close(&self, params: DidCloseTextDocumentParams) {
		self.files.handle_close(params);
	}

	#[instrument(skip_all, fields(uri =% params.text_document_position_params.text_document.uri))]
	async fn goto_definition(
		&self,
		params: GotoDefinitionParams,
	) -> LspResult<Option<GotoDefinitionResponse>> {
		self.provide_definition(params).await
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
