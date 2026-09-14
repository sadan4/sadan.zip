pub mod cmds;
mod custom;
mod definition;
mod doc;
mod reference;
mod hover;

use std::{borrow::Cow, debug_assert_matches, path::PathBuf, sync::Arc};

use anyhow::Result;
use ast_parser::get_offset_from_line_and_column;
use tower_lsp::{
	Client, LanguageServer, async_trait, jsonrpc, lsp_types::{
		DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, Location, OneOf, Position, ReferenceParams, SaveOptions, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
	},
};
use tracing::{error, instrument, warn};
use webpack_ast_parser::ThreadSafeParser;

use crate::{
	JValue,
	LspResult,
	ReloadHandle,
	SERVER_NAME,
	SERVER_VERSION,
	State,
	module_cache::SplitModuleCache,
	wss::WsServer,
};

pub struct Server {
	pub(crate) client: Client,
	files: doc::Files,
	state: Arc<State>,
	log_reload_handle: Option<ReloadHandle>,
	module_cache: SplitModuleCache,
}

#[must_use = "this is just a builder for the server"]
pub struct ServerBuilder {
	files: doc::Files,
	state: Arc<State>,
	log_reload_handle: Option<ReloadHandle>,
	module_cache: SplitModuleCache,
}

impl ServerBuilder {
	pub fn new() -> Result<Self> {
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
		let module_cache = SplitModuleCache::new(state.ws.clone());
		Ok(Self {
			files,
			state,
			log_reload_handle: None,
			module_cache,
		})
	}

	pub fn with_reload_handle(mut self, handle: ReloadHandle) -> Self {
		debug_assert_matches!(
			self.log_reload_handle,
			None,
			"reload handle already set"
		);
		self.log_reload_handle = Some(handle);
		self
	}
	#[must_use]
	pub fn build(self, client: Client) -> Server {
		self.module_cache
			.set_client(client.clone());
		Server {
			client,
			files: self.files,
			state: self.state,
			log_reload_handle: self.log_reload_handle,
			module_cache: self.module_cache,
		}
	}
}

fn workspace_roots(params: &InitializeParams) -> Vec<PathBuf> {
	let folders = params
		.workspace_folders
		.iter()
		.flatten()
		.map(|folder| &folder.uri);
	// deprecated in the spec, but plenty of clients still only send this
	let root_uri = params.root_uri.iter();
	folders
		.chain(root_uri)
		.filter_map(|uri| {
			uri.to_file_path()
				.inspect_err(|()| {
					warn!(%uri, "Workspace folder is not a file path, ignoring");
				})
				.ok()
		})
		.collect()
}

fn cursor_offset(
	doc_text: &str,
	parser: &ThreadSafeParser,
	position: Position,
) -> LspResult<u32> {
	if &**parser.get_source() != doc_text {
		return Err(jsonrpc::Error {
			code: jsonrpc::ErrorCode::ContentModified,
			message: Cow::Borrowed(
				"parser source does not match the open document, skipping; save the file to refresh it",
			),
			data: None,
		});
	}
	Ok(get_offset_from_line_and_column(
		doc_text,
		position.line,
		position.character,
	))
}

impl Server {}

#[async_trait]
impl LanguageServer for Server {
	async fn initialize(
		&self,
		params: InitializeParams,
	) -> LspResult<InitializeResult> {
		self.module_cache
			.set_workspace_roots(workspace_roots(&params));
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
				references_provider: Some(OneOf::Left(true)),
				hover_provider: Some(HoverProviderCapability::Simple(true)),
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
	async fn did_save(&self, params: DidSaveTextDocumentParams) {
		// the parser and dep graph we cached are from the old contents
		self.module_cache
			.handle_save(&params.text_document.uri)
			.await;
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

	async fn references(
		&self,
		params: ReferenceParams,
	) -> LspResult<Option<Vec<Location>>> {
		self.gen_references(params).await
	}

	async fn execute_command(
		&self,
		params: ExecuteCommandParams,
	) -> LspResult<Option<JValue>> {
		self.handle_cmd(params).await
	}

	async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
		self.provide_hover(params).await
	}

	async fn shutdown(&self) -> LspResult<()> {
		todo!()
	}
}
