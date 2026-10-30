//! Forwards scrape and parse progress to the client as MCP progress
//! notifications

use std::{
	borrow::Cow,
	sync::{
		Arc,
		atomic::{AtomicU64, AtomicUsize, Ordering},
	},
};

use discord_scraper::{NoProgress, ScrapeProgress};
use rmcp::{
	Peer,
	RoleServer,
	model::{ProgressNotificationParam, ProgressToken},
	service::RequestContext,
};
use term_progress::Stage;
use tokio::runtime::Handle;

pub struct McpProgress {
	peer: Peer<RoleServer>,
	token: ProgressToken,
	handle: Handle,
	/// must increase with every notification
	progress: AtomicU64,
	chunk_total: AtomicUsize,
	chunks_done: AtomicUsize,
}

impl McpProgress {
	/// Reports to the caller of `ctx` if it asked for progress, otherwise
	/// reports nothing
	pub fn for_request(
		ctx: &RequestContext<RoleServer>,
	) -> Arc<dyn ScrapeProgress> {
		match (ctx.meta.get_progress_token(), Handle::try_current()) {
			(Some(token), Ok(handle)) => Arc::new(Self {
				peer: ctx.peer.clone(),
				token,
				handle,
				progress: AtomicU64::new(0),
				chunk_total: AtomicUsize::new(0),
				chunks_done: AtomicUsize::new(0),
			}),
			_ => Arc::new(NoProgress),
		}
	}

	fn send(&self, message: Option<String>, total: Option<usize>) {
		let progress = self
			.progress
			.fetch_add(1, Ordering::Relaxed)
			+ 1;
		#[expect(
			clippy::cast_precision_loss,
			reason = "progress counts are nowhere near 2^52"
		)]
		let mut param =
			ProgressNotificationParam::new(self.token.clone(), progress as f64);
		param.message = message;
		#[expect(
			clippy::cast_precision_loss,
			reason = "progress counts are nowhere near 2^52"
		)]
		if let Some(total) = total {
			param = param.with_total(total as f64);
		}
		let peer = self.peer.clone();
		self.handle.spawn(async move {
			_ = peer.notify_progress(param).await;
		});
	}
}

impl ScrapeProgress for McpProgress {
	fn set_stage(&self, msg: &'static str) {
		self.send(Some(msg.to_owned()), None);
	}

	fn set_chunk_total(&self, total: usize) {
		self.chunk_total
			.store(total, Ordering::Relaxed);
		self.send(Some(format!("Fetching {total} chunks")), None);
	}

	fn chunk_finished(&self) {
		let done = self
			.chunks_done
			.fetch_add(1, Ordering::Relaxed)
			+ 1;
		let total = self.chunk_total.load(Ordering::Relaxed);
		// one notification per ~5% so a big build doesn't flood the client
		let step = (total / 20).max(1);
		if done.is_multiple_of(step) || done == total {
			self.send(Some(format!("Fetched {done}/{total} chunks")), None);
		}
	}

	fn make_stage(&self, msg: Cow<'static, str>, n: Option<usize>) -> Stage {
		let msg = match n {
			Some(n) => format!("{msg} ({n})"),
			None => msg.into_owned(),
		};
		self.send(Some(msg), None);
		Stage::hidden()
	}
}
