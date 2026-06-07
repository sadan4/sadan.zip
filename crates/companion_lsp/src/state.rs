use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};

use dashmap::DashMap;
use tokio::sync::{RwLock, oneshot};
use tower_lsp::lsp_types::Url;

use crate::{
	discord_bridge::DiscordBridge,
	lsp::{cross_module::CrossModuleData, patch_helper::Registry as PatchHelperRegistry},
	module_cache::ModuleCache,
};

/// Server-side correlation for `vencord/quickPick` round-trips. The client
/// replies via `vencord/quickPick/response` (custom client→server method),
/// which resolves the matching oneshot.
#[derive(Default)]
pub struct QuickPickPending {
	counter: AtomicU64,
	pending: DashMap<u64, oneshot::Sender<Option<String>>>,
}

impl QuickPickPending {
	pub fn register(&self) -> (u64, oneshot::Receiver<Option<String>>) {
		let nonce = self.counter.fetch_add(1, Ordering::Relaxed);
		let (tx, rx) = oneshot::channel();
		self.pending.insert(nonce, tx);
		(nonce, rx)
	}

	pub fn resolve(&self, nonce: u64, selected: Option<String>) {
		if let Some((_, tx)) = self.pending.remove(&nonce) {
			let _ = tx.send(selected);
		}
	}

	pub fn drop_pending(&self, nonce: u64) {
		self.pending.remove(&nonce);
	}
}

/// In-memory representation of a text document the editor has opened.
#[derive(Debug, Clone)]
pub struct Document {
	pub uri: Url,
	pub version: i32,
	pub language_id: String,
	pub text: String,
}

#[derive(Default)]
pub struct SessionState {
	pub documents: DashMap<Url, Document>,
	/// Bridge owns its own per-field locking; no outer `RwLock` here.
	pub discord: DiscordBridge,
	pub module_cache: RwLock<ModuleCache>,
	/// Cached precomputed view of the `.modules/` directory: every
	/// source loaded, plus the inverse dep graph. Built lazily on the
	/// first references request; invalidated when the on-disk module
	/// cache changes (download / purge).
	pub cross_module_data: RwLock<Option<Arc<CrossModuleData>>>,
	/// hashedKey -> localized string. Populated lazily by i18n hover.
	pub i18n_cache: DashMap<String, String>,
	/// Outstanding QuickPick round-trips waiting on the editor.
	pub quick_picks: QuickPickPending,
	/// Active Patch Helper sessions, indexed by the URI of the plugin
	/// source they're tracking. Populated when `vencord.openPatchHelper`
	/// fires; dropped on source close or when relocation fails.
	pub patch_helpers: PatchHelperRegistry,
}

impl SessionState {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get_document(&self, uri: &Url) -> Option<Document> {
		self.documents.get(uri).map(|entry| entry.value().clone())
	}
}

pub type SharedState = Arc<SessionState>;
