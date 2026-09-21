pub mod cmds;
pub mod custom;
mod definition;
mod diagnostics;
mod doc;
mod hover;
mod lenses;
mod patch_helper2;
mod reference;
mod test_patch;

use std::{borrow::Cow, debug_assert_matches, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result};
use ast_parser::pool::AllocPool;
use tokio::sync::mpsc;
use tower_lsp_server::{
	Client,
	LanguageServer,
	jsonrpc,
	ls_types::{
		CodeLens,
		CodeLensOptions,
		CodeLensParams,
		DidChangeTextDocumentParams,
		DidCloseTextDocumentParams,
		DidOpenTextDocumentParams,
		DidSaveTextDocumentParams,
		ExecuteCommandParams,
		GotoDefinitionParams,
		GotoDefinitionResponse,
		Hover,
		HoverParams,
		HoverProviderCapability,
		InitializeParams,
		InitializeResult,
		Location,
		OneOf,
		Position,
		ReferenceParams,
		SaveOptions,
		ServerCapabilities,
		ServerInfo,
		ShowDocumentParams,
		TextDocumentSyncCapability,
		TextDocumentSyncKind,
		TextDocumentSyncOptions,
		TextDocumentSyncSaveOptions,
		request::ShowDocument,
	},
};
use tracing::{debug, error, instrument, warn};
use webpack_ast_parser::ThreadSafeParser;

use crate::{
	JValue,
	LspResult,
	ReloadHandle,
	SERVER_NAME,
	SERVER_VERSION,
	State,
	lsp::{
		custom::{Ephemera, ephemera::EphemeralChange},
		diagnostics::Diagnostics,
	},
	module_cache::SplitModuleCache,
	util::uri,
	wss::WsServer,
};

pub struct Server {
	pub(crate) client: Client,
	files: doc::Files,
	state: Arc<State>,
	log_reload_handle: Option<ReloadHandle>,
	module_cache: SplitModuleCache,
	ephemera: Ephemera,
	diagnostics: Diagnostics,
	patch_helpers: patch_helper2::Helpers,
	pool: Arc<AllocPool>,
}

#[must_use = "this is just a builder for the server"]
pub struct ServerBuilder {
	files: doc::Files,
	state: Arc<State>,
	log_reload_handle: Option<ReloadHandle>,
	module_cache: SplitModuleCache,
	ephemera: Ephemera,
	ephemera_changes: mpsc::UnboundedReceiver<EphemeralChange>,
	diagnostics: Diagnostics,
	diagnostic_requests: mpsc::UnboundedReceiver<diagnostics::Request>,
	patch_helpers: patch_helper2::Helpers,
	pool: Arc<AllocPool>,
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
		let (ephemera, ephemera_changes) = Ephemera::new();
		let (diagnostics, diagnostic_requests) = Diagnostics::new();
		let patch_helpers = patch_helper2::Helpers::default();
		let pool = Arc::new(AllocPool::new(None));
		Ok(Self {
			files,
			state,
			log_reload_handle: None,
			module_cache,
			ephemera,
			ephemera_changes,
			diagnostics,
			diagnostic_requests,
			patch_helpers,
			pool,
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
		Ephemera::serve_changes(client.clone(), self.ephemera_changes);
		Diagnostics::serve(
			client.clone(),
			self.files.clone(),
			self.diagnostic_requests,
			self.pool.clone(),
			self.state.ws.clone(),
		);
		Server {
			client,
			files: self.files,
			state: self.state,
			log_reload_handle: self.log_reload_handle,
			module_cache: self.module_cache,
			ephemera: self.ephemera,
			diagnostics: self.diagnostics,
			patch_helpers: self.patch_helpers,
			pool: self.pool,
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
	#[expect(deprecated)]
	let root_uri = params.root_uri.iter();
	folders
		.chain(root_uri)
		.filter_map(|uri| {
			uri::to_path(uri)
				.inspect_err(|_| {
					warn!(uri =% uri.as_str(), "Workspace folder is not a file path, ignoring");
				})
				.ok()
				.map(Cow::into_owned)
		})
		.collect()
}

fn cursor_offset(
	doc: &doc::Document,
	parser: &ThreadSafeParser,
	position: Position,
) -> LspResult<u32> {
	if &**parser.get_source() != doc.text.as_str() {
		return Err(jsonrpc::Error {
			code: jsonrpc::ErrorCode::ContentModified,
			message: Cow::Borrowed(
				"parser source does not match the open document, skipping; save the file to refresh it",
			),
			data: None,
		});
	}
	Ok(doc.offset_at(position))
}

impl Server {
	async fn show_document(&self, params: ShowDocumentParams) -> Result<()> {
		self.client
			.send_request::<ShowDocument>(params)
			.await
			.context("Failed to show document")?
			.success
			.then_some(())
			.context("Client reported failing to show document")
	}
}

#[allow(clippy::unused_async_trait_impl)]
impl LanguageServer for Server {
	async fn initialize(
		&self,
		params: InitializeParams,
	) -> LspResult<InitializeResult> {
		debug!("client capabilities: {:#?}", params.capabilities);
		self.module_cache
			.set_workspace_roots(workspace_roots(&params));
		let position_encoding = self
			.files
			.negotiate_encoding(params.capabilities.general.as_ref());
		Ok(InitializeResult {
			capabilities: ServerCapabilities {
				position_encoding: Some(position_encoding),
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
				code_lens_provider: Some(CodeLensOptions {
					resolve_provider: Some(false),
				}),
				execute_command_provider: Some(Self::get_cmd_provider()),
				..ServerCapabilities::default()
			},
			server_info: Some(ServerInfo {
				name: String::from(SERVER_NAME),
				version: Some(String::from(SERVER_VERSION)),
			}),
			offset_encoding: None,
		})
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri.as_str()))]
	async fn did_open(&self, params: DidOpenTextDocumentParams) {
		let uri = params.text_document.uri.clone();
		self.files.handle_open(params);
		self.diagnostics_open_hook(uri);
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri.as_str()))]
	async fn did_change(&self, params: DidChangeTextDocumentParams) {
		let uri = params.text_document.uri.clone();
		self.files.handle_change(params);
		if let Err(e) = self
			.patch_helper_change_hook(&uri)
			.await
		{
			warn!("Failed to run patch helper change hook {e:?}");
		}
		self.diagnostics_change_hook(uri);
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri.as_str()))]
	async fn did_save(&self, params: DidSaveTextDocumentParams) {
		// the parser and dep graph we cached are from the old contents
		self.module_cache
			.handle_save(&params.text_document.uri)
			.await;
	}

	#[instrument(skip_all, fields(uri =% params.text_document.uri.as_str()))]
	async fn did_close(&self, params: DidCloseTextDocumentParams) {
		self.patch_helper_close_hook(&params.text_document.uri);
		self.diagnostics_close_hook(params.text_document.uri.clone());
		self.files.handle_close(params);
	}

	#[instrument(skip_all, fields(uri =% params.text_document_position_params.text_document.uri.as_str()))]
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

	async fn code_lens(
		&self,
		params: CodeLensParams,
	) -> LspResult<Option<Vec<CodeLens>>> {
		self.provide_lenses(params).await
	}

	async fn shutdown(&self) -> LspResult<()> {
		todo!()
	}
}
