use std::time::Duration;

use crate::{SERVER_NAME, lsp};
use anyhow::{Result, anyhow};
use const_format::formatc;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use tokio::time::timeout;
use tower_lsp::lsp_types::request::Request;
use tracing::{instrument, warn};

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QuickPickRequest {
	pub items: Vec<SmolStr>,
	pub placeholder: Option<SmolStr>,
	pub allow_free_text: bool,
}

type QuickPickResponse = Option<String>;

struct QuickPickMessage;

impl QuickPickMessage {
	/// The user might take a while to respond
	const TIMEOUT: Duration = Duration::from_mins(5);
}

impl Request for QuickPickMessage {
	type Params = QuickPickRequest;
	type Result = QuickPickResponse;
	const METHOD: &'static str = formatc!("$/{SERVER_NAME}/quick_pick");
}

impl lsp::Server {
	#[instrument(level = "error", skip(self, req), fields(question =? req.placeholder))]
	pub(crate) async fn quick_pick(
		&self,
		req: QuickPickRequest,
	) -> Result<Option<String>> {
		use QuickPickMessage as M;
		let send_req = self.client.send_request::<M>(req);
		match timeout(M::TIMEOUT, send_req).await {
			Ok(Ok(ret)) => Ok(ret),
			Ok(Err(guh)) => {
				warn!("Request failed: {guh}");
				Err(anyhow!("Request failed: {guh}"))
			}
			Err(_) => {
				warn!("Request timed out");
				Ok(None)
			}
		}
	}
}
