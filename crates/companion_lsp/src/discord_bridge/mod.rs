use std::{cmp::Ordering, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{RwLock, mpsc},
};
use tokio_tungstenite::{
	accept_hdr_async,
	tungstenite::{
		Message,
		handshake::server::{ErrorResponse, Request, Response},
		http::StatusCode,
		protocol::{CloseFrame, frame::coding::CloseCode},
	},
};
use tracing::{debug, error, info, trace, warn};

pub mod messages;
pub mod rpc;

use crate::state::SharedState;

use messages::{
	IncomingFrame,
	ModuleListData,
	OutgoingKind,
	VersionRequestData,
	VersionResponseData,
};
use rpc::{NonceCounter, PendingMap, RpcSender, VERSION_TIMEOUT};

/// Default port for the Discord-side bridge. Matches the legacy TS server.
pub const BRIDGE_PORT: u16 = 8485;

/// Allowed origins for incoming bridge connections.
pub const ALLOWED_ORIGINS: &[&str] = &[
	"https://discord.com",
	"https://canary.discord.com",
	"https://ptb.discord.com",
];

/// Minimum vc-userDevTools client version we accept (major, minor, patch).
pub const MIN_CLIENT_VERSION: (u32, u32, u32) = (0, 1, 1);

/// Our own reported version when the client asks via the version handshake.
/// In future we should source this from `CARGO_PKG_VERSION`; the wire
/// protocol just wants a semver tuple.
pub const SERVER_VERSION: (u32, u32, u32) = (1, 0, 0);

/// Shared handle to whatever Discord client is currently connected. When no
/// client is connected, the inner `RpcSender` is in its `disconnected` state
/// and any RPC call returns a clear error pointing users at the userplugin
/// readme.
pub struct DiscordBridge {
	sender: RwLock<RpcSender>,
	module_cache: RwLock<Vec<String>>,
}

impl Default for DiscordBridge {
	fn default() -> Self {
		Self::new()
	}
}

impl DiscordBridge {
	pub fn new() -> Self {
		Self {
			sender: RwLock::new(RpcSender::disconnected()),
			module_cache: RwLock::new(Vec::new()),
		}
	}

	pub async fn sender(&self) -> RpcSender {
		self.sender.read().await.clone()
	}

	pub async fn is_connected(&self) -> bool {
		self.sender.read().await.is_connected()
	}

	pub async fn module_cache_snapshot(&self) -> Vec<String> {
		self.module_cache.read().await.clone()
	}

	async fn set_connected(&self, sender: RpcSender) {
		*self.sender.write().await = sender;
	}

	async fn set_disconnected(&self) {
		*self.sender.write().await = RpcSender::disconnected();
	}

	async fn update_module_cache(&self, modules: Vec<String>) {
		*self.module_cache.write().await = modules;
	}

	// ------------------------------------------------------------------
	// High-level RPC helpers used by LSP handlers
	// ------------------------------------------------------------------

	/// `testPatch` — returns Ok(()) when the patch applies cleanly,
	/// otherwise Err containing the Discord-side error message.
	pub async fn test_patch(&self, data: messages::PatchData) -> Result<()> {
		let sender = self.sender().await;
		sender
			.request(
				messages::OutgoingKind::TestPatch { data },
				rpc::DEFAULT_TIMEOUT,
			)
			.await
			.map(drop)
	}

	/// `testFind` — see [`test_patch`].
	pub async fn test_find(&self, data: messages::FindData) -> Result<()> {
		let sender = self.sender().await;
		sender
			.request(
				messages::OutgoingKind::TestFind { data },
				rpc::DEFAULT_TIMEOUT,
			)
			.await
			.map(drop)
	}

	/// `i18n` — given a 6-char hashed key, returns the localized string.
	pub async fn i18n_lookup(&self, hashed_key: &str) -> Result<String> {
		let sender = self.sender().await;
		let frame = sender
			.request(
				messages::OutgoingKind::I18n {
					data: messages::I18nLookupData {
						hashed_key: hashed_key.to_owned(),
					},
				},
				rpc::DEFAULT_TIMEOUT,
			)
			.await?;
		let value: messages::I18nValueData = frame.parse_data()?;
		Ok(value.value)
	}
}

/// Spawn point used from `main`. Binds to `BRIDGE_PORT` and serves one
/// active Discord connection at a time (matches the legacy server's
/// "last-write-wins" behavior).
pub async fn run(state: SharedState) -> Result<()> {
	let addr: SocketAddr = ([127, 0, 0, 1], BRIDGE_PORT).into();
	let listener = TcpListener::bind(addr)
		.await
		.with_context(|| format!("failed to bind discord bridge to {addr}"))?;
	info!(%addr, "discord bridge listening");

	loop {
		let (stream, peer) = match listener.accept().await {
			Ok(pair) => pair,
			Err(e) => {
				error!(?e, "accept failed; retrying");
				continue;
			}
		};
		let state = state.clone();
		tokio::spawn(async move {
			if let Err(e) = handle_connection(state, stream, peer).await {
				warn!(?peer, ?e, "discord bridge connection ended with error");
			}
		});
	}
}

async fn handle_connection(
	state: SharedState,
	stream: TcpStream,
	peer: SocketAddr,
) -> Result<()> {
	// Origin allow-listing happens during the websocket handshake.
	let ws = accept_hdr_async(stream, origin_check)
		.await
		.with_context(|| format!("ws handshake failed from {peer}"))?;
	info!(?peer, "discord client connected");

	let (mut sink, mut stream_rx) = ws.split();
	let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<String>();

	let pending = PendingMap::new();
	let nonces = Arc::new(NonceCounter::default());
	let sender = RpcSender {
		outbound: Some(outbound_tx),
		pending: pending.clone(),
		nonces: nonces.clone(),
	};

	state
		.discord
		.set_connected(sender.clone())
		.await;

	// Outbound pump: messages from RpcSender::request -> websocket frames.
	let outbound_task = tokio::spawn(async move {
		while let Some(msg) = outbound_rx.recv().await {
			trace!(bytes = msg.len(), "ws send");
			if sink
				.send(Message::Text(msg))
				.await
				.is_err()
			{
				break;
			}
		}
		let _ = sink
			.send(Message::Close(Some(CloseFrame {
				code: CloseCode::Normal,
				reason: "server shutdown".into(),
			})))
			.await;
	});

	// Version handshake. If the client is too old we close with the same
	// policy-violation code the legacy server used (1008).
	let version_check = tokio::spawn({
		let sender = sender.clone();
		let state = state.clone();
		async move { perform_version_handshake(sender, state).await }
	});

	// Inbound pump: dispatch each frame.
	while let Some(frame) = stream_rx.next().await {
		match frame {
			Ok(Message::Text(text)) => {
				dispatch_text(&state, &pending, &text).await;
			}
			Ok(Message::Binary(bytes)) => {
				if let Ok(text) = std::str::from_utf8(&bytes) {
					dispatch_text(&state, &pending, text).await;
				} else {
					warn!("ignoring non-utf8 binary ws frame");
				}
			}
			Ok(Message::Close(_)) => {
				debug!("client sent close frame");
				break;
			}
			Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
			Err(e) => {
				warn!(?e, "ws stream error");
				break;
			}
		}
	}

	info!(?peer, "discord client disconnected");
	state.discord.set_disconnected().await;
	outbound_task.abort();
	let _ = version_check.await;
	Ok(())
}

async fn dispatch_text(state: &SharedState, pending: &PendingMap, text: &str) {
	trace!(text, "ws recv");
	let frame = match serde_json::from_str::<IncomingFrame>(text) {
		Ok(f) => f,
		Err(e) => {
			warn!(?e, raw = text, "could not decode incoming ws frame");
			return;
		}
	};

	// Unsolicited moduleList updates feed the in-memory cache used by
	// QuickPick handlers, even when no request is outstanding.
	if frame.kind == "moduleList"
		&& frame.ok
		&& let Ok(payload) = frame.parse_data::<ModuleListData>()
	{
		debug!(count = payload.modules.len(), "moduleList push");
		state
			.discord
			.update_module_cache(payload.modules)
			.await;
	}

	if !pending.deliver(frame) {
		// Either an unsolicited frame we don't care about (already handled
		// above), or a response whose nonce has already timed out — both
		// are non-fatal.
		trace!("dropped incoming frame with no matching pending request");
	}
}

#[expect(clippy::result_large_err)]
fn origin_check(
	req: &Request,
	response: Response,
) -> Result<Response, ErrorResponse> {
	let Some(origin_hdr) = req.headers().get("Origin") else {
		// No Origin header = native client. Allow.
		return Ok(response);
	};
	let origin = origin_hdr.to_str().unwrap_or_default();
	if ALLOWED_ORIGINS.contains(&origin) {
		Ok(response)
	} else {
		warn!(%origin, "rejected ws connection from disallowed origin");
		let mut err = ErrorResponse::new(Some("Origin not allowed".to_owned()));
		*err.status_mut() = StatusCode::FORBIDDEN;
		Err(err)
	}
}

async fn perform_version_handshake(sender: RpcSender, state: SharedState) {
	let req = OutgoingKind::Version {
		data: VersionRequestData {
			server_version: SERVER_VERSION,
		},
	};
	let frame = match sender
		.request(req, VERSION_TIMEOUT)
		.await
	{
		Ok(f) => f,
		Err(e) => {
			warn!(?e, "version handshake failed");
			return;
		}
	};
	let payload: VersionResponseData = match frame.parse_data() {
		Ok(p) => p,
		Err(e) => {
			warn!(?e, "could not parse version response");
			return;
		}
	};
	if is_outdated(payload.client_version, MIN_CLIENT_VERSION) {
		warn!(
			?payload.client_version,
			min = ?MIN_CLIENT_VERSION,
			"vc-userDevTools client is outdated; some features will fail"
		);
	} else {
		info!(version = ?payload.client_version, "discord client version OK");
	}
	let _ = state;
}

/// Returns true when `actual < min` componentwise.
fn is_outdated(actual: (u32, u32, u32), min: (u32, u32, u32)) -> bool {
	match actual.0.cmp(&min.0) {
		Ordering::Less => true,
		Ordering::Greater => false,
		Ordering::Equal => match actual.1.cmp(&min.1) {
			Ordering::Less => true,
			Ordering::Greater => false,
			Ordering::Equal => actual.2 < min.2,
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio_tungstenite::tungstenite::client::IntoClientRequest;

	#[test]
	fn version_ordering_works() {
		assert!(is_outdated((0, 1, 0), (0, 1, 1)));
		assert!(is_outdated((0, 0, 9), (0, 1, 0)));
		assert!(!is_outdated((0, 1, 1), (0, 1, 1)));
		assert!(!is_outdated((1, 0, 0), (0, 9, 9)));
		assert!(!is_outdated((0, 2, 0), (0, 1, 9)));
	}

	/// Spins up the bridge on an ephemeral port, connects a mock ws client,
	/// exchanges a testPatch RPC, and verifies the round-trip.
	#[tokio::test]
	async fn rpc_round_trip_over_real_socket() {
		let state = Arc::new(crate::state::SessionState::new());
		let listener = TcpListener::bind("127.0.0.1:0")
			.await
			.unwrap();
		let addr = listener.local_addr().unwrap();

		// Accept exactly one connection in a task.
		let server_state = state.clone();
		let server_task = tokio::spawn(async move {
			let (stream, peer) = listener.accept().await.unwrap();
			handle_connection(server_state, stream, peer).await
		});

		// Mock client: connect, then drive the round-trip.
		let url = format!("ws://{addr}/");
		let req = url.into_client_request().unwrap();
		let (mut client, _) = tokio_tungstenite::connect_async(req)
			.await
			.unwrap();

		// Wait until the server records the connection.
		for _ in 0..50 {
			if state.discord.is_connected().await {
				break;
			}
			tokio::time::sleep(Duration::from_millis(20)).await;
		}
		assert!(state.discord.is_connected().await);

		// Issue an RPC from the server side; the mock client will reply.
		let sender = state.discord.sender().await;
		let rpc_task = tokio::spawn(async move {
			sender
				.request(
					OutgoingKind::TestPatch {
						data: messages::PatchData {
							find_type: messages::FindType::String,
							find: "q".into(),
							replacement: vec![],
						},
					},
					Duration::from_millis(500),
				)
				.await
		});

		// Read the outgoing frame the bridge sent us, echo back ok with the
		// same nonce.
		let outgoing = match client.next().await.unwrap().unwrap() {
			Message::Text(t) => t,
			other => panic!("unexpected ws frame: {other:?}"),
		};
		let parsed: serde_json::Value =
			serde_json::from_str(&outgoing).unwrap();
		let nonce = parsed["nonce"].as_u64().unwrap();

		// The very first message will likely be the version handshake, NOT
		// our testPatch. Reply ok to whatever we got, then read the next.
		let first_type = parsed["type"]
			.as_str()
			.unwrap()
			.to_owned();
		let reply = serde_json::json!({
			"type": first_type,
			"nonce": nonce,
			"ok": true,
			"data": if first_type == "version" {
				serde_json::json!({ "clientVersion": [1, 0, 0] })
			} else {
				serde_json::Value::Null
			},
		});
		client
			.send(Message::Text(reply.to_string()))
			.await
			.unwrap();

		// If we just answered the version handshake, read the actual
		// testPatch frame.
		if first_type == "version" {
			let outgoing = match client.next().await.unwrap().unwrap() {
				Message::Text(t) => t,
				other => panic!("unexpected ws frame: {other:?}"),
			};
			let parsed: serde_json::Value =
				serde_json::from_str(&outgoing).unwrap();
			assert_eq!(parsed["type"], "testPatch");
			let nonce = parsed["nonce"].as_u64().unwrap();
			let reply = serde_json::json!({
				"type": "testPatch",
				"nonce": nonce,
				"ok": true,
				"data": null,
			});
			client
				.send(Message::Text(reply.to_string()))
				.await
				.unwrap();
		}

		let got = rpc_task.await.unwrap().unwrap();
		assert!(got.ok);
		assert_eq!(got.kind, "testPatch");

		// Tear down.
		drop(client);
		let _ =
			tokio::time::timeout(Duration::from_millis(500), server_task).await;

		assert!(!state.discord.is_connected().await);
	}

	#[tokio::test]
	async fn module_list_push_updates_cache() {
		let state = Arc::new(crate::state::SessionState::new());
		let listener = TcpListener::bind("127.0.0.1:0")
			.await
			.unwrap();
		let addr = listener.local_addr().unwrap();

		let server_state = state.clone();
		let server_task = tokio::spawn(async move {
			let (stream, peer) = listener.accept().await.unwrap();
			handle_connection(server_state, stream, peer).await
		});

		let url = format!("ws://{addr}/");
		let req = url.into_client_request().unwrap();
		let (mut client, _) = tokio_tungstenite::connect_async(req)
			.await
			.unwrap();

		// Wait for connect bookkeeping.
		for _ in 0..50 {
			if state.discord.is_connected().await {
				break;
			}
			tokio::time::sleep(Duration::from_millis(20)).await;
		}

		client
			.send(Message::Text(
				serde_json::json!({
					"type": "moduleList",
					"ok": true,
					"data": { "modules": ["111", "222", "333"] }
				})
				.to_string(),
			))
			.await
			.unwrap();

		// Let the dispatcher run.
		for _ in 0..50 {
			let cache = state
				.discord
				.module_cache_snapshot()
				.await;
			if !cache.is_empty() {
				assert_eq!(cache, vec!["111", "222", "333"]);
				break;
			}
			tokio::time::sleep(Duration::from_millis(20)).await;
		}

		drop(client);
		let _ =
			tokio::time::timeout(Duration::from_millis(500), server_task).await;
	}
}
