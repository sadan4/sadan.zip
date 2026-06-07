use std::sync::Arc;

use tower_lsp::{
	Client,
	LanguageServer,
	jsonrpc::Result as LspResult,
	lsp_types::{
		CodeLens,
		CodeLensOptions,
		CodeLensParams,
		DidChangeTextDocumentParams,
		DidCloseTextDocumentParams,
		DidOpenTextDocumentParams,
		ExecuteCommandOptions,
		ExecuteCommandParams,
		GotoDefinitionParams,
		GotoDefinitionResponse,
		Hover,
		HoverParams,
		HoverProviderCapability,
		InitializeParams,
		InitializeResult,
		InitializedParams,
		Location,
		MessageType,
		OneOf,
		ReferenceParams,
		ServerCapabilities,
		ServerInfo,
		TextDocumentSyncCapability,
		TextDocumentSyncKind,
		WorkDoneProgressOptions,
	},
};

use crate::{
	state::{Document, SharedState},
	vencord_ext,
};

mod client_ext;
mod code_lens;
mod commands;
pub mod cross_module;
mod definition;
pub mod diagnostics;
mod document;
mod hover;
pub mod patch_helper;
mod references;

pub struct Backend {
	pub client: Client,
	pub state: SharedState,
}

impl Backend {
	pub const fn new(client: Client, state: SharedState) -> Self {
		Self { client, state }
	}

	/// Custom server method: editor delivers a `QuickPick` selection back to a
	/// pending server-initiated request. Implementation lives in `commands`.
	pub async fn on_quick_pick_response(
		&self,
		params: serde_json::Value,
	) -> LspResult<serde_json::Value> {
		commands::on_quick_pick_response(self, params).await
	}
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
	async fn initialize(
		&self,
		params: InitializeParams,
	) -> LspResult<InitializeResult> {
		// Capture the workspace root so the module cache can orient itself.
		// Prefer the modern `workspaceFolders` field;
		// fall back to the deprecated `rootUri` so older clients keep
		// working.
		#[allow(deprecated)]
		let workspace_root = params
			.workspace_folders
			.as_ref()
			.and_then(|f| f.first())
			.and_then(|f| f.uri.to_file_path().ok())
			.or_else(|| {
				params
					.root_uri
					.as_ref()
					.and_then(|u| u.to_file_path().ok())
			});

		if let Some(root) = workspace_root {
			let mut cache = self.state.module_cache.write().await;
			cache.set_workspace_root(&root);
			let _ = cache.rescan().await;
		}

		let capabilities = ServerCapabilities {
			text_document_sync: Some(TextDocumentSyncCapability::Kind(
				TextDocumentSyncKind::INCREMENTAL,
			)),
			hover_provider: Some(HoverProviderCapability::Simple(true)),
			definition_provider: Some(OneOf::Left(true)),
			references_provider: Some(OneOf::Left(true)),
			code_lens_provider: Some(CodeLensOptions {
				resolve_provider: Some(false),
			}),
			execute_command_provider: Some(ExecuteCommandOptions {
				commands: vencord_ext::ALL_COMMAND_IDS
					.iter()
					.map(|s| (*s).to_owned())
					.collect(),
				work_done_progress_options: WorkDoneProgressOptions::default(),
			}),
			..Default::default()
		};

		Ok(InitializeResult {
			capabilities,
			server_info: Some(ServerInfo {
				name: env!("CARGO_PKG_NAME").to_owned(),
				version: Some(env!("CARGO_PKG_VERSION").to_owned()),
			}),
		})
	}

	async fn initialized(&self, _params: InitializedParams) {
		self.client
			.log_message(MessageType::INFO, "companion_lsp ready")
			.await;
	}

	async fn shutdown(&self) -> LspResult<()> {
		Ok(())
	}

	async fn did_open(&self, params: DidOpenTextDocumentParams) {
		document::on_did_open(self, params);
	}

	async fn did_change(&self, params: DidChangeTextDocumentParams) {
		document::on_did_change(self, params);
	}

	async fn did_close(&self, params: DidCloseTextDocumentParams) {
		document::on_did_close(self, params);
	}

	async fn hover(
		&self,
		params: HoverParams,
	) -> LspResult<Option<Hover>> {
		hover::hover(self, params).await
	}

	async fn goto_definition(
		&self,
		params: GotoDefinitionParams,
	) -> LspResult<Option<GotoDefinitionResponse>> {
		definition::goto_definition(self, params).await
	}

	async fn references(
		&self,
		params: ReferenceParams,
	) -> LspResult<Option<Vec<Location>>> {
		references::references(self, params).await
	}

	async fn code_lens(
		&self,
		params: CodeLensParams,
	) -> LspResult<Option<Vec<CodeLens>>> {
		code_lens::code_lens(self, params).await
	}

	async fn execute_command(
		&self,
		params: ExecuteCommandParams,
	) -> LspResult<Option<serde_json::Value>> {
		commands::execute_command(self, params).await
	}
}

/// Helper used by hover/definition/etc. — fetches the document or returns None.
#[allow(dead_code)] // used by phase 2+ handlers as they fill in
pub fn get_doc(
	state: &SharedState,
	uri: &tower_lsp::lsp_types::Url,
) -> Option<Document> {
	state.get_document(uri)
}

const _: fn() = || {
	const fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<Backend>();
	assert_send_sync::<Arc<Backend>>();
};
