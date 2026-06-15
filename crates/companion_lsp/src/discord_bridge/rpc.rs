//! Nonce-based request/response correlation for the Discord WebSocket bridge.
//!
//! Each outgoing message is tagged with a monotonically increasing `nonce`.
//! When the Discord client responds, we look the nonce up in a pending-map
//! and deliver the parsed frame to a `oneshot` channel. Unsolicited frames
//! (notably `moduleList`) are routed to a separate broadcast handler.

use std::{
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result, anyhow};
use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};

use super::messages::{IncomingFrame, OutgoingFrame, OutgoingKind};

/// Default timeout for a single RPC. Matches `VencordCompanion`'s legacy 5s.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5_000);
/// Longer timeout used only for the initial version handshake.
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(30);

/// Handle handed to active connections so they can register outgoing requests
/// and receive matched responses. Cheap to clone (`Arc`-shaped internally).
#[derive(Clone, Default)]
pub struct PendingMap {
	inner: Arc<DashMap<u64, oneshot::Sender<IncomingFrame>>>,
}

impl PendingMap {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn register(&self, nonce: u64) -> oneshot::Receiver<IncomingFrame> {
		let (tx, rx) = oneshot::channel();
		self.inner.insert(nonce, tx);
		rx
	}

	pub fn drop_pending(&self, nonce: u64) {
		self.inner.remove(&nonce);
	}

	/// Delivers an incoming frame to a pending request. Returns true if a
	/// matching pending request existed and was resolved.
	pub fn deliver(&self, frame: IncomingFrame) -> bool {
		let Some(nonce) = frame.nonce else {
			return false;
		};
		if let Some((_, tx)) = self.inner.remove(&nonce) {
			let _ = tx.send(frame);
			true
		} else {
			false
		}
	}

	pub fn len(&self) -> usize {
		self.inner.len()
	}

	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}
}

/// Bumped per outgoing frame. The legacy TS server started at 8485 (the
/// same value as the port, presumably for ease of debugging). We mirror that
/// so that captured packet logs from old and new servers look similar.
#[derive(Debug)]
pub struct NonceCounter(AtomicU64);

impl Default for NonceCounter {
	fn default() -> Self {
		Self(AtomicU64::new(8485))
	}
}

impl NonceCounter {
	pub fn next(&self) -> u64 {
		self.0.fetch_add(1, Ordering::Relaxed)
	}
}

/// Sender side handed to call sites. A `None` `outbound` means no client is
/// connected; callers should surface a clear error.
#[derive(Clone)]
pub struct RpcSender {
	pub outbound: Option<mpsc::UnboundedSender<String>>,
	pub pending:  PendingMap,
	pub nonces:   Arc<NonceCounter>,
}

impl RpcSender {
	pub fn disconnected() -> Self {
		Self {
			outbound: None,
			pending:  PendingMap::new(),
			nonces:   Arc::new(NonceCounter::default()),
		}
	}

	pub const fn is_connected(&self) -> bool {
		self.outbound.is_some()
	}

	/// Sends an outgoing frame and awaits the matching response.
	pub async fn request(
		&self,
		kind: OutgoingKind,
		timeout: Duration,
	) -> Result<IncomingFrame> {
		let tx = self
			.outbound
			.as_ref()
			.ok_or_else(|| anyhow!(
				"No Discord client connected. Make sure Discord is open with \
				 the vc-userDevTools plugin enabled."
			))?;

		let nonce = self.nonces.next();
		let frame = OutgoingFrame { kind, nonce };
		let json = serde_json::to_string(&frame)
			.context("failed to encode outgoing frame")?;

		let rx = self.pending.register(nonce);
		tx.send(json)
			.map_err(|_| {
				self.pending.drop_pending(nonce);
				anyhow!("Discord connection closed before send")
			})?;

		match tokio::time::timeout(timeout, rx).await {
			Ok(Ok(frame)) if frame.ok => Ok(frame),
			Ok(Ok(frame)) => Err(frame.into_error()),
			Ok(Err(_)) => Err(anyhow!("response channel dropped")),
			Err(_) => {
				self.pending.drop_pending(nonce);
				Err(anyhow!(
					"timed out waiting for Discord client response (nonce {nonce})"
				))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::discord_bridge::messages::{
		FindData,
		FindNode,
		PatchData,
		FindType,
		MatchNode,
		ReplaceNode,
		Replacement,
	};

	fn dummy_patch() -> PatchData {
		PatchData {
			find_type:   FindType::String,
			find:        "x".into(),
			replacement: vec![Replacement {
				match_:  MatchNode::String { value: "a".into() },
				replace: ReplaceNode::String { value: "b".into() },
			}],
		}
	}

	#[tokio::test]
	async fn disconnected_sender_errors() {
		let sender = RpcSender::disconnected();
		let res = sender
			.request(
				OutgoingKind::TestPatch { data: dummy_patch() },
				Duration::from_millis(50),
			)
			.await;
		assert!(res.is_err());
		let msg = res.unwrap_err().to_string();
		assert!(msg.contains("No Discord client"), "unexpected: {msg}");
	}

	#[tokio::test]
	async fn nonce_correlation_resolves_oneshot() {
		let pending = PendingMap::new();
		let nonce = 100;
		let rx = pending.register(nonce);
		assert_eq!(pending.len(), 1);

		let frame: IncomingFrame = serde_json::from_value(serde_json::json!({
			"type": "testPatch",
			"ok": true,
			"nonce": nonce,
			"data": null,
		}))
		.unwrap();
		assert!(pending.deliver(frame));
		let got = rx.await.unwrap();
		assert!(got.ok);
		assert!(pending.is_empty());
	}

	#[tokio::test]
	async fn unsolicited_frame_does_not_resolve_anything() {
		let pending = PendingMap::new();
		let frame: IncomingFrame = serde_json::from_value(serde_json::json!({
			"type": "moduleList",
			"ok": true,
			"data": { "modules": [] },
		}))
		.unwrap();
		assert!(!pending.deliver(frame));
	}

	#[tokio::test]
	async fn timeout_drops_pending_entry() {
		let (tx, mut rx) = mpsc::unbounded_channel::<String>();
		let sender = RpcSender {
			outbound: Some(tx),
			pending:  PendingMap::new(),
			nonces:   Arc::new(NonceCounter::default()),
		};

		let send_task = tokio::spawn({
			let sender = sender.clone();
			async move {
				sender
					.request(
						OutgoingKind::TestFind {
							data: FindData {
								kind: "findByProps".into(),
								args: vec![FindNode::String { value: "x".into() }],
							},
						},
						Duration::from_millis(40),
					)
					.await
			}
		});

		// Drain whatever the sender writes out, but never respond.
		let _ = rx.recv().await;

		let result = send_task.await.unwrap();
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("timed out"));
		assert!(sender.pending.is_empty(), "pending entry should be cleared");
	}

	#[tokio::test]
	async fn full_round_trip_via_pending_map() {
		let (tx, mut outbound_rx) = mpsc::unbounded_channel::<String>();
		let sender = RpcSender {
			outbound: Some(tx),
			pending:  PendingMap::new(),
			nonces:   Arc::new(NonceCounter::default()),
		};

		let send = tokio::spawn({
			let sender = sender.clone();
			async move {
				sender
					.request(
						OutgoingKind::TestPatch { data: dummy_patch() },
						Duration::from_millis(500),
					)
					.await
			}
		});

		let wire = outbound_rx.recv().await.unwrap();
		let parsed: serde_json::Value = serde_json::from_str(&wire).unwrap();
		let nonce = parsed["nonce"].as_u64().unwrap();

		let reply: IncomingFrame = serde_json::from_value(serde_json::json!({
			"type": "testPatch",
			"ok": true,
			"nonce": nonce,
			"data": null,
		}))
		.unwrap();
		assert!(sender.pending.deliver(reply));

		let got = send.await.unwrap().unwrap();
		assert!(got.ok);
		assert_eq!(got.nonce, Some(nonce));
	}
}
