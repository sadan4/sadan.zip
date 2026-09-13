use std::{
	debug_assert_matches,
	sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

use tokio::{sync::mpsc, task::JoinHandle};
use tower_lsp::{
	Client,
	lsp_types::{
		ProgressParams,
		ProgressParamsValue,
		ProgressToken,
		WorkDoneProgress,
		WorkDoneProgressBegin,
		WorkDoneProgressCreateParams,
		WorkDoneProgressEnd,
		WorkDoneProgressReport,
		notification::Progress,
		request::WorkDoneProgressCreate,
	},
};
use tracing::{debug, error, warn};

use crate::lsp;

static NEXT_TOKEN: AtomicI32 = AtomicI32::new(1);

pub struct OwnedProgressHandle {
	tx: mpsc::UnboundedSender<WorkDoneProgressReport>,
	id: ProgressToken,
	dispatcher: JoinHandle<()>,
	client: Client,
	drop: AtomicBool,
}

impl Drop for OwnedProgressHandle {
	fn drop(&mut self) {
		if !self.drop.load(Ordering::Relaxed) {
			return;
		}
		let ProgressToken::Number(id) = self.id else {
			unreachable!("only number tokens are used");
		};
		let client = self.client.clone();
		self.dispatcher.abort();
		tokio::spawn(async move {
			let params = ProgressParams {
				token: ProgressToken::Number(id),
				value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
					WorkDoneProgressEnd {
						message: Some("Progress dropped".to_owned()),
					},
				)),
			};
			client
				.send_notification::<Progress>(params)
				.await;
		});
	}
}

fn mint_token() -> ProgressToken {
	let id = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
	ProgressToken::Number(id)
}

impl OwnedProgressHandle {
	pub fn step(
		&self,
		percentage: impl Into<Option<u32>>,
		msg: impl Into<Option<String>>,
	) {
		let mut percentage = percentage.into();
		if let Some(p) = &mut percentage {
			debug_assert_matches!(
				p,
				0..=100,
				"percentage must be between 0 and 100"
			);
			*p = (*p).clamp(0, 100);
		}
		let report = WorkDoneProgressReport {
			percentage,
			message: msg.into(),
			..Default::default()
		};
		if let Err(e) = self.tx.send(report) {
			warn!("Failed to send progress report: {e}");
		}
	}
}

impl lsp::Server {
	/// This is **not** async, it spawns a task to handle progress reporting
	pub fn start_progress(
		client: Client,
		params: WorkDoneProgressBegin,
	) -> OwnedProgressHandle {
		debug_assert_matches!(
			params.cancellable,
			None | Some(false),
			"TODO: handle cancellable progress"
		);
		let token = mint_token();
		let (tx, mut rx) = mpsc::unbounded_channel();
		let client2 = client.clone();
		// spawn the dispatcher early so that the client can continue as soon as possible
		let token2 = token.clone();
		let dispatcher = tokio::spawn(async move {
			let client = client2;
			let token = token2;
			// create it
			if let Err(e) = client
				.send_request::<WorkDoneProgressCreate>(
					WorkDoneProgressCreateParams {
						token: token.clone(),
					},
				)
				.await
			{
				error!(%params.title, "Failed to create progress, {e}");
				return;
			}
			// start it
			client
				.send_notification::<Progress>(ProgressParams {
					token: token.clone(),
					value: ProgressParamsValue::WorkDone(
						WorkDoneProgress::Begin(params),
					),
				})
				.await;
			while let Some(report) = rx.recv().await {
				client
					.send_notification::<Progress>(ProgressParams {
						token: token.clone(),
						value: ProgressParamsValue::WorkDone(
							WorkDoneProgress::Report(report),
						),
					})
					.await;
			}
			debug!("Progress dispatcher for {token:?} finished");
		});

		OwnedProgressHandle {
			tx,
			id: token,
			dispatcher,
			client,
			drop: AtomicBool::new(true),
		}
	}
}
