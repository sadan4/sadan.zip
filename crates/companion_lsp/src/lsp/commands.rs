use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tower_lsp::{
	jsonrpc::{Error as LspError, Result as LspResult},
	lsp_types::{ExecuteCommandParams, MessageType},
};

use crate::{
	discord_bridge::messages::{
		DiffModuleData,
		DisablePluginData,
		ExtractModuleData,
		FindData,
		IncomingFrame,
		OutgoingKind,
		PatchData,
	},
	lsp::{Backend, client_ext},
	vencord_ext::{
		CMD_DIFF_MODULE,
		CMD_DISABLE_PLUGIN,
		CMD_DOWNLOAD_MODULE_CACHE,
		CMD_EXTRACT_FIND,
		CMD_EXTRACT_MODULE,
		CMD_OPEN_PATCH_HELPER,
		CMD_PURGE_MODULE_CACHE,
		CMD_TEST_FIND,
		CMD_TEST_PATCH,
		CMD_WEBPACK_I18N_HOVER_COPY,
	},
};

// ---------------------------------------------------------------------------
// Wire payloads (server -> Discord bridge) and command result shapes.
//
// `OutgoingKind::{Extract,Diff}` carry an untyped `serde_json::Value`, and the
// editor expects an untyped result `Value` back from `executeCommand`. We model
// both ends as structs here and serialize at the boundary so the shapes live in
// one place instead of scattered `json!` literals.
// ---------------------------------------------------------------------------

/// Discriminant for the bridge's extract/diff requests. Only `id`-based lookups
/// are issued from here; serializes to `"id"`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum ExtractType {
	Id,
}

/// `{ extractType: "id", idOrSearch: <n>, usePatched: null }`
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractByIdRequest {
	extract_type: ExtractType,
	id_or_search: i64,
	use_patched: Option<bool>,
}

impl ExtractByIdRequest {
	const fn new(id: i64) -> Self {
		Self {
			extract_type: ExtractType::Id,
			id_or_search: id,
			use_patched: None,
		}
	}
}

/// `{ extractType: "id", idOrSearch: <n> }`
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiffByIdRequest {
	extract_type: ExtractType,
	id_or_search: i64,
}

/// Generic `{ ok: true }` acknowledgement returned to the editor.
#[derive(Serialize)]
struct OkResponse {
	ok: bool,
}

/// Result of `downloadModuleCache`.
#[derive(Serialize)]
struct DownloadResponse {
	downloaded: usize,
	failed: usize,
	total: usize,
}

/// Result of `extractModule` / `diffModule`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleSummary {
	module_number: i64,
	patched_by: Vec<String>,
}

/// Result of `extractFind`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractFindResponse {
	module_number: i64,
	find: bool,
}

/// Serialize a command result struct into the untyped `Value` the editor wants.
/// Serialization of these plain structs is infallible in practice.
fn to_result<T: Serialize>(value: T) -> Result<Value> {
	serde_json::to_value(value).map_err(Into::into)
}

fn ok_response() -> Result<Value> {
	to_result(OkResponse { ok: true })
}

pub async fn execute_command(
	backend: &Backend,
	params: ExecuteCommandParams,
) -> LspResult<Option<Value>> {
	let result = match params.command.as_str() {
		CMD_TEST_PATCH => cmd_test_patch(backend, params.arguments).await,
		CMD_TEST_FIND => cmd_test_find(backend, params.arguments).await,
		CMD_DISABLE_PLUGIN => {
			cmd_disable_plugin(backend, params.arguments).await
		}
		CMD_DOWNLOAD_MODULE_CACHE => cmd_download_module_cache(backend).await,
		CMD_PURGE_MODULE_CACHE => cmd_purge_module_cache(backend).await,
		CMD_EXTRACT_MODULE => {
			cmd_extract_module(backend, params.arguments).await
		}
		CMD_EXTRACT_FIND => cmd_extract_find(backend, params.arguments).await,
		CMD_DIFF_MODULE => cmd_diff_module(backend, params.arguments).await,
		CMD_WEBPACK_I18N_HOVER_COPY => {
			// Pure client-side action — return the payload so the editor
			// shim can stuff it onto the clipboard.
			Ok(params
				.arguments
				.first()
				.cloned()
				.unwrap_or(Value::Null))
		}
		CMD_OPEN_PATCH_HELPER => {
			crate::lsp::patch_helper::open(backend, params.arguments).await
		}
		other => {
			tracing::warn!(command = %other, "unknown executeCommand");
			return Err(LspError::method_not_found());
		}
	};

	match result {
		Ok(v) => Ok(Some(v)),
		Err(e) => {
			// Surface the failure to the user as a clean toast and return Ok
			// to the editor. Returning a JSON-RPC error here causes both
			// vscode-languageclient and VSCode itself to pop their own
			// generic "Request workspace/executeCommand failed" notification,
			// which doubles up with our friendly message.
			tracing::debug!(?e, command = %params.command, "executeCommand failed");
			backend
				.client
				.show_message(MessageType::ERROR, e.to_string())
				.await;
			Ok(None)
		}
	}
}

/// Custom server method: client returns the user's `QuickPick` selection.
pub fn on_quick_pick_response(
	backend: &Backend,
	params: Value,
) -> LspResult<Value> {
	#[derive(Deserialize)]
	struct Resp {
		nonce: u64,
		selected: Option<String>,
	}
	let resp: Resp = serde_json::from_value(params).map_err(|e| LspError {
		code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
		message: e.to_string().into(),
		data: None,
	})?;
	backend
		.state
		.quick_picks
		.resolve(resp.nonce, resp.selected);
	Ok(Value::Null)
}

// ---------------------------------------------------------------------------
// Bridge passthroughs
// ---------------------------------------------------------------------------

async fn cmd_test_patch(backend: &Backend, args: Vec<Value>) -> Result<Value> {
	let data = resolve_patch_data(backend, &args)?;
	backend
		.state
		.discord
		.test_patch(data)
		.await?;
	backend
		.client
		.show_message(MessageType::INFO, "Patch applied successfully.")
		.await;
	ok_response()
}

/// Accepts either:
/// - `{ uri, patchIndex }` (the code-lens shape) — `patchIndex` is the index
///   into `VencordAstParser::patches()`. We re-parse the document and pick
///   the Nth wire patch in source order, matching how `code_lens.rs` numbers
///   them.
/// - a fully-formed wire `PatchData` — used by callers that already have one
///   in hand (e.g. tests, future Patch Helper).
fn resolve_patch_data(backend: &Backend, args: &[Value]) -> Result<PatchData> {
	#[derive(Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct LensArg {
		uri: String,
		patch_index: usize,
	}

	let first = args
		.first()
		.context("testPatch requires at least one argument")?;

	if let Ok(lens) = serde_json::from_value::<LensArg>(first.clone()) {
		let url = tower_lsp::lsp_types::Url::parse(&lens.uri)
			.with_context(|| format!("invalid uri {:?}", lens.uri))?;
		let doc = backend
			.state
			.get_document(&url)
			.with_context(|| format!("no open document for {url}"))?;
		let mut wire = crate::lsp::diagnostics::extract_patches(&doc.text);
		if lens.patch_index >= wire.len() {
			anyhow::bail!(
				"patch index {} out of range (parser saw {} patches)",
				lens.patch_index,
				wire.len(),
			);
		}
		return wire
			.get_mut(lens.patch_index)
			.and_then(Option::take)
			.map(|(_, data)| data)
			.context(
				"this patch can't be tested over the wire yet (function \
				 replacements aren't supported)",
			);
	}

	serde_json::from_value(first.clone())
		.context("expected {uri, patchIndex} or PatchData")
}

async fn cmd_test_find(backend: &Backend, args: Vec<Value>) -> Result<Value> {
	let data: FindData = parse_first_arg(&args)?;
	backend
		.state
		.discord
		.test_find(data)
		.await?;
	backend
		.client
		.show_message(MessageType::INFO, "Find resolved successfully.")
		.await;
	ok_response()
}

async fn cmd_disable_plugin(
	backend: &Backend,
	args: Vec<Value>,
) -> Result<Value> {
	let data: DisablePluginData = parse_first_arg(&args)?;
	let sender = backend.state.discord.sender().await;
	sender
		.request(
			crate::discord_bridge::messages::OutgoingKind::Disable { data },
			crate::discord_bridge::rpc::DEFAULT_TIMEOUT,
		)
		.await?;
	ok_response()
}

// ---------------------------------------------------------------------------
// Module cache lifecycle
// ---------------------------------------------------------------------------

async fn cmd_purge_module_cache(backend: &Backend) -> Result<Value> {
	backend
		.state
		.module_cache
		.write()
		.await
		.purge()
		.await?;
	// The precomputed cross-module data is now stale — drop it so the
	// next references request rebuilds against the empty cache.
	*backend
		.state
		.cross_module_data
		.write()
		.await = None;
	backend
		.client
		.show_message(MessageType::INFO, "Module cache purged.")
		.await;
	ok_response()
}

async fn cmd_download_module_cache(backend: &Backend) -> Result<Value> {
	if !backend
		.state
		.discord
		.is_connected()
		.await
	{
		anyhow::bail!(
			"No Discord client connected. Open Discord and enable vc-userDevTools."
		);
	}
	let progress = client_ext::begin_work_progress(
		&backend.client,
		"Loading all modules",
		None,
	)
	.await;

	// Ask the bridge to push the current moduleList. Discord pushes this
	// unsolicitedly on connect too, but we re-request here in case ids have
	// shifted (e.g. after a Discord client rebuild).
	let sender = backend.state.discord.sender().await;
	let _ = sender
		.request(
			crate::discord_bridge::messages::OutgoingKind::AllModules {
				data: (),
			},
			crate::discord_bridge::rpc::DEFAULT_TIMEOUT,
		)
		.await;

	let ids = backend
		.state
		.discord
		.module_cache_snapshot()
		.await;
	if ids.is_empty() {
		anyhow::bail!("Discord client returned no modules");
	}

	if let Some(p) = progress {
		p.end(None).await;
	}

	let total = ids.len();
	let mut ok = 0usize;
	let mut err = 0usize;

	let progress = Arc::new(
		client_ext::begin_work_progress(
			&backend.client,
			"Downloading modules",
			Some(format!("0 / {total}")),
		)
		.await,
	);
	let (tx, mut rx) = mpsc::channel(total);
	let handle = tokio::spawn({
		let progress = progress.clone();
		async move {
			if let Some(p) = progress.as_ref() {
				let mut done = 0;
				while rx.recv().await == Some(()) {
					done += 1;
					let pct = ((done as u64 * 100) / total as u64) as u32;
					p.report(format!("{done} / {total}"), pct)
						.await;
				}
			}
		}
	});

	// Cap the number of in-flight Extract requests on the WS bridge, but let
	// the post-response work (pretty-print + disk write) run unbounded — those
	// stages are local-CPU/disk-bound and the bridge is the actual scarce
	// resource we want to throttle.
	const REQUEST_CONCURRENCY: usize = 8;
	let mut store_tasks = Vec::with_capacity(total);
	{
		let mut fetched = stream::iter(ids)
			.map(|id| {
				let state = backend.state.clone();
				async move {
					let res = fetch_module(&state, &id).await;
					(id, res)
				}
			})
			.buffer_unordered(REQUEST_CONCURRENCY);

		while let Some((id, fetch_res)) = fetched.next().await {
			match fetch_res {
				Ok(payload) => {
					let state = backend.state.clone();
					let tx = tx.clone();
					store_tasks.push(tokio::spawn(async move {
						let res =
							store_module(&state, &id, payload.parse_data()?)
								.await;
						_ = tx.send(()).await;
						Ok((id, res))
					}));
				}
				Err(e) => {
					err += 1;
					tracing::debug!(id = %id, ?e, "module download failed");
				}
			}
		}
	}

	for task in store_tasks {
		match task
			.await
			.map_err(|e| anyhow!(e))
			.flatten()
		{
			Ok((_, Ok(()))) => ok += 1,
			Ok((id, Err(e))) => {
				err += 1;
				tracing::debug!(id = %id, ?e, "module store failed");
			}
			Err(e) => {
				err += 1;
				tracing::debug!(?e, "store task panicked");
			}
		}
	}

	handle.abort();

	if let Some(p) = progress.as_ref() {
		p.end(Some(format!("Downloaded {ok} of {total} ({err} failed)")))
			.await;
	}

	// New modules on disk → eagerly rebuild the cross-module view so
	// `_cache.json` is persisted now (instead of waiting for the first
	// references request) and the next cold start hits the cache.
	let cache_root = backend
		.state
		.module_cache
		.read()
		.await
		.root()
		.map(std::path::Path::to_owned);

	if let Some(root) = cache_root {
		match tokio::task::spawn_blocking(move || {
			crate::lsp::cross_module::CrossModuleData::build(root)
		})
		.await
		{
			Ok(Ok(data)) => {
				*backend
					.state
					.cross_module_data
					.write()
					.await = Some(Arc::new(data));
			}
			Ok(Err(e)) => {
				tracing::debug!(?e, "post-download cross-module build failed");
				*backend
					.state
					.cross_module_data
					.write()
					.await = None;
			}
			Err(e) => {
				tracing::debug!(
					?e,
					"post-download cross-module build panicked"
				);
				*backend
					.state
					.cross_module_data
					.write()
					.await = None;
			}
		}
	} else {
		*backend
			.state
			.cross_module_data
			.write()
			.await = None;
	}

	to_result(DownloadResponse {
		downloaded: ok,
		failed: err,
		total,
	})
}

async fn fetch_module(
	state: &crate::state::SharedState,
	id: &str,
) -> Result<IncomingFrame> {
	let sender = state.discord.sender().await;
	let id_num: i64 = id
		.parse()
		.with_context(|| format!("module id {id} is not numeric"))?;

	sender
		.request(
			crate::discord_bridge::messages::OutgoingKind::Extract {
				data: serde_json::to_value(ExtractByIdRequest::new(id_num))?,
			},
			crate::discord_bridge::rpc::DEFAULT_TIMEOUT,
		)
		.await
}

async fn store_module(
	state: &crate::state::SharedState,
	id: &str,
	payload: crate::discord_bridge::messages::ExtractModuleData,
) -> Result<()> {
	// The Discord client side already pretty-prints with the same algorithm
	// our `pretty_printer` uses, so re-formatting is usually a no-op; do it
	// anyway to defend against older clients that send raw sources.
	let formatted = pretty_printer::format_to_str(&payload.module, 4)
		.unwrap_or(payload.module);

	state
		.module_cache
		.write()
		.await
		.write_module(id, &formatted)
		.await?;
	Ok(())
}

// ---------------------------------------------------------------------------
// Module extract / diff (with QuickPick fallback for missing args)
// ---------------------------------------------------------------------------

async fn cmd_extract_module(
	backend: &Backend,
	args: Vec<Value>,
) -> Result<Value> {
	let id = resolve_module_id(backend, &args, "Module ID to extract").await?;
	let payload = bridge_extract_by_id(backend, id).await?;
	let path = save_to_cache(backend, &payload).await?;
	let uri =
		tower_lsp::lsp_types::Url::from_file_path(&path).map_err(|()| {
			anyhow::anyhow!("could not URI-encode {}", path.display())
		})?;
	client_ext::request_show_document(&backend.client, uri, true)
		.await
		.map_err(|e| anyhow::anyhow!("showDocument: {e:?}"))?;
	to_result(ModuleSummary {
		module_number: payload.module_number,
		patched_by: payload.patched_by,
	})
}

async fn cmd_extract_find(
	backend: &Backend,
	args: Vec<Value>,
) -> Result<Value> {
	let data = args
		.first()
		.cloned()
		.context("extractFind requires a payload")?;
	let sender = backend.state.discord.sender().await;
	let frame = sender
		.request(
			OutgoingKind::Extract { data },
			crate::discord_bridge::rpc::DEFAULT_TIMEOUT,
		)
		.await?;
	let payload: ExtractModuleData = frame.parse_data()?;
	let path = save_to_cache(backend, &payload).await?;
	let uri =
		tower_lsp::lsp_types::Url::from_file_path(&path).map_err(|()| {
			anyhow::anyhow!("could not URI-encode {}", path.display())
		})?;
	client_ext::request_show_document(&backend.client, uri, true)
		.await
		.map_err(|e| anyhow::anyhow!("showDocument: {e:?}"))?;
	to_result(ExtractFindResponse {
		module_number: payload.module_number,
		find: payload.find.unwrap_or(false),
	})
}

async fn cmd_diff_module(backend: &Backend, args: Vec<Value>) -> Result<Value> {
	let id = resolve_module_id(backend, &args, "Module ID to diff").await?;
	let sender = backend.state.discord.sender().await;
	let frame = sender
		.request(
			OutgoingKind::Diff {
				data: serde_json::to_value(DiffByIdRequest {
					extract_type: ExtractType::Id,
					id_or_search: id,
				})?,
			},
			crate::discord_bridge::rpc::DEFAULT_TIMEOUT,
		)
		.await?;
	let payload: DiffModuleData = frame.parse_data()?;

	// Persist both halves to disk so the editor can open them with stable URIs.
	let cache_root = backend
		.state
		.module_cache
		.read()
		.await
		.root()
		.map(std::path::Path::to_owned)
		.context("no module cache root configured")?;
	let diff_dir = cache_root.join(".diff");
	tokio::fs::create_dir_all(&diff_dir)
		.await
		.with_context(|| format!("create_dir_all({})", diff_dir.display()))?;
	let src_path = diff_dir.join(format!("{id}.source.js"));
	let pat_path = diff_dir.join(format!("{id}.patched.js"));
	tokio::fs::write(&src_path, &payload.source).await?;
	tokio::fs::write(&pat_path, &payload.patched).await?;

	let left =
		tower_lsp::lsp_types::Url::from_file_path(&src_path).map_err(|()| {
			anyhow::anyhow!("could not URI-encode {}", src_path.display())
		})?;
	let right =
		tower_lsp::lsp_types::Url::from_file_path(&pat_path).map_err(|()| {
			anyhow::anyhow!("could not URI-encode {}", pat_path.display())
		})?;
	client_ext::request_show_diff(
		&backend.client,
		left,
		right,
		&format!("Module {id}: source ↔ patched"),
	)
	.await?;

	to_result(ModuleSummary {
		module_number: payload.module_number,
		patched_by: payload.patched_by,
	})
}

async fn resolve_module_id(
	backend: &Backend,
	args: &[Value],
	placeholder: &str,
) -> Result<i64> {
	if let Some(first) = args.first() {
		// Either {idOrSearch: N} from a code lens, or a bare integer.
		if let Some(n) = first.as_i64() {
			return Ok(n);
		}
		if let Some(obj) = first.as_object() {
			if let Some(n) = obj
				.get("idOrSearch")
				.and_then(Value::as_i64)
			{
				return Ok(n);
			}
			if let Some(n) = obj.get("id").and_then(Value::as_i64) {
				return Ok(n);
			}
		}
	}

	let items = backend
		.state
		.discord
		.module_cache_snapshot()
		.await;
	let pick = client_ext::request_quick_pick(
		&backend.client,
		&backend.state,
		items,
		placeholder,
		true,
	)
	.await?
	.ok_or_else(|| anyhow::anyhow!("no module selected"))?;
	pick.parse::<i64>()
		.with_context(|| format!("module id {pick:?} is not numeric"))
}

async fn bridge_extract_by_id(
	backend: &Backend,
	id: i64,
) -> Result<ExtractModuleData> {
	let sender = backend.state.discord.sender().await;
	let frame = sender
		.request(
			OutgoingKind::Extract {
				data: serde_json::to_value(ExtractByIdRequest::new(id))?,
			},
			crate::discord_bridge::rpc::DEFAULT_TIMEOUT,
		)
		.await?;
	frame.parse_data()
}

async fn save_to_cache(
	backend: &Backend,
	payload: &ExtractModuleData,
) -> Result<std::path::PathBuf> {
	let formatted = pretty_printer::format_to_str(&payload.module, 4)
		.unwrap_or_else(|_| payload.module.clone());
	let id_str = payload.module_number.to_string();
	let path = backend
		.state
		.module_cache
		.write()
		.await
		.write_module(&id_str, &formatted)
		.await?;
	Ok(path)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_first_arg<T: for<'de> Deserialize<'de>>(args: &[Value]) -> Result<T> {
	let v = args
		.first()
		.context("command requires at least one argument")?;
	serde_json::from_value(v.clone()).map_err(Into::into)
}
