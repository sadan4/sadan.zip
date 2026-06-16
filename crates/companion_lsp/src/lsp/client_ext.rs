//! Helpers that issue server→client requests in the `vencord/*` namespace.
//! Editors that don't implement the custom methods will return `method not
//! found`; callers downgrade gracefully when that happens.

use std::{
	mem,
	sync::{
		OnceLock,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_lsp::{
	Client,
	jsonrpc::Result as LspResult,
	lsp_types::{
		ProgressParams,
		ProgressParamsValue,
		ProgressToken,
		ShowDocumentParams,
		Url,
		WorkDoneProgress,
		WorkDoneProgressBegin,
		WorkDoneProgressCreateParams,
		WorkDoneProgressEnd,
		WorkDoneProgressReport,
		notification::Progress,
		request::{Request, ShowDocument, WorkDoneProgressCreate},
	},
};

use crate::{state::SharedState, vencord_ext};

/// Wait at most this long for the editor to respond to a custom request.
const CLIENT_REQUEST_TIMEOUT: Duration = Duration::from_mins(1);

/// Parameters for the custom `vencord/quickPick` request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuickPickParams {
	nonce: String,
	items: Vec<String>,
	placeholder: String,
	allow_free_text: bool,
}

/// Ask the editor to pop a `QuickPick`. Returns whatever the user selected, or
/// `None` if they dismissed it.
pub async fn request_quick_pick(
	client: &Client,
	state: &SharedState,
	items: Vec<String>,
	placeholder: &str,
	allow_free_text: bool,
) -> Result<Option<String>> {
	let (nonce, rx) = state.quick_picks.register();

	struct QuickPick;
	impl Request for QuickPick {
		type Params = QuickPickParams;
		type Result = Value;
		const METHOD: &'static str = vencord_ext::QUICK_PICK_METHOD;
	}

	let send = client.send_request::<QuickPick>(QuickPickParams {
		nonce: nonce.to_string(),
		items,
		placeholder: placeholder.to_owned(),
		allow_free_text,
	});

	// The server-side handler in `commands::on_quick_pick_response` resolves
	// the oneshot; if the editor doesn't implement the method we'll see an
	// error here and clean up the pending entry.
	match tokio::time::timeout(CLIENT_REQUEST_TIMEOUT, send).await {
		Ok(Ok(_)) => {
			match tokio::time::timeout(CLIENT_REQUEST_TIMEOUT, rx).await {
				Ok(Ok(s)) => Ok(s),
				Ok(Err(_)) => {
					state.quick_picks.drop_pending(nonce);
					Err(anyhow!("QuickPick response channel dropped"))
				}
				Err(_) => {
					state.quick_picks.drop_pending(nonce);
					Err(anyhow!("QuickPick response timed out"))
				}
			}
		}
		Ok(Err(e)) => {
			state.quick_picks.drop_pending(nonce);
			Err(anyhow!("editor does not support vencord/quickPick: {e}"))
		}
		Err(_) => {
			state.quick_picks.drop_pending(nonce);
			Err(anyhow!("QuickPick send timed out"))
		}
	}
}

/// Standard LSP `window/showDocument` — opens a file URI in the editor.
pub async fn request_show_document(
	client: &Client,
	uri: Url,
	take_focus: bool,
) -> LspResult<()> {
	client
		.send_request::<ShowDocument>(ShowDocumentParams {
			uri,
			external: Some(false),
			take_focus: Some(take_focus),
			selection: None,
		})
		.await?;
	Ok(())
}

/// A single side of a `vencord/showDiff` request: `{ uri }`.
#[derive(Serialize, Deserialize)]
struct DiffUri {
	uri: String,
}

/// Parameters for the custom `vencord/showDiff` request.
#[derive(Serialize, Deserialize)]
struct ShowDiffParams {
	left: DiffUri,
	right: DiffUri,
	title: String,
}

/// Custom `vencord/showDiff` request — asks the editor to open a side-by-side
/// diff view of two URIs.
pub async fn request_show_diff(
	client: &Client,
	left: Url,
	right: Url,
	title: &str,
) -> Result<()> {
	struct ShowDiff;
	impl Request for ShowDiff {
		type Params = ShowDiffParams;
		type Result = Value;
		const METHOD: &'static str = vencord_ext::SHOW_DIFF_METHOD;
	}

	client
		.send_request::<ShowDiff>(ShowDiffParams {
			left: DiffUri {
				uri: left.to_string(),
			},
			right: DiffUri {
				uri: right.to_string(),
			},
			title: title.to_owned(),
		})
		.await
		.map(drop)
		.map_err(|e| anyhow!("editor does not support vencord/showDiff: {e}"))
}

/// A live LSP work-done progress handle. Drop without calling `end` and the
/// progress bar stays open in the editor, so always call `end` (or let it
/// happen via the `Drop` impl which fires a best-effort `end` notification).
pub struct WorkProgress {
	client: Option<Client>,
	token: ProgressToken,
	done: OnceLock<()>,
}

impl WorkProgress {
	const fn dummy() -> Self {
		Self {
			client: None,
			token: ProgressToken::Number(0),
			done: OnceLock::new(),
		}
	}

	pub async fn report(&self, message: impl Into<String>, percentage: u32) {
		if self.done.get().is_some() {
			return;
		}
		self.client
			.as_ref()
			.unwrap()
			.send_notification::<Progress>(ProgressParams {
				token: self.token.clone(),
				value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
					WorkDoneProgressReport {
						cancellable: Some(false),
						message: Some(message.into()),
						percentage: Some(percentage),
					},
				)),
			})
			.await;
	}

	pub async fn end(&self, message: Option<String>) {
		if self.done.get().is_some() {
			return;
		}
		_ = self.done.set(());
		self.client
			.as_ref()
			.unwrap()
			.send_notification::<Progress>(ProgressParams {
				token: self.token.clone(),
				value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
					WorkDoneProgressEnd { message },
				)),
			})
			.await;
	}
}

/// Ask the editor to register a work-done progress token, then send the
/// `begin` notification. If the editor doesn't support
/// `window/workDoneProgress/create`, returns `None` and callers continue
/// without progress reporting.
pub async fn begin_work_progress(
	client: &Client,
	title: impl Into<String>,
	initial_message: Option<String>,
) -> Option<WorkProgress> {
	static NEXT_ID: AtomicU64 = AtomicU64::new(1);
	let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
	let token = ProgressToken::Number(
		i32::try_from(id).unwrap_or((id & 0x7fff_ffff) as i32),
	);

	if let Err(e) = client
		.send_request::<WorkDoneProgressCreate>(WorkDoneProgressCreateParams {
			token: token.clone(),
		})
		.await
	{
		tracing::debug!(?e, "client does not support workDoneProgress/create");
		return None;
	}

	client
		.send_notification::<Progress>(ProgressParams {
			token: token.clone(),
			value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
				WorkDoneProgressBegin {
					title: title.into(),
					cancellable: Some(false),
					message: initial_message,
					percentage: Some(0),
				},
			)),
		})
		.await;

	Some(WorkProgress {
		client: Some(client.clone()),
		token,
		done: OnceLock::new(),
	})
}

impl Drop for WorkProgress {
	fn drop(&mut self) {
		let this = mem::replace(self, Self::dummy());
		tokio::spawn(async move {
			this.end(None).await;
		});
	}
}
