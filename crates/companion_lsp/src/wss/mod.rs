use std::{
	net::SocketAddr,
	sync::{
		Arc,
		atomic::{AtomicU32, Ordering},
	},
	time::Duration,
};

use anyhow::{Context as _, Result, anyhow, bail};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt as _};
use serde::Deserialize;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{RwLock, mpsc, oneshot},
	task::JoinHandle,
	time::timeout,
};
use tokio_tungstenite::{
	accept_hdr_async,
	tungstenite::{
		Message,
		handshake::server::{ErrorResponse, Request, Response},
		http::StatusCode,
	},
};
use tracing::{debug, error, info, trace, warn};

use crate::wss::types::{MsgFromClient, MsgToClient};

#[derive(Clone)]
pub struct WsServer(Arc<RwLock<Inner>>);

const NO_CONN_MSG: &str = "No Discord client connected. Make sure Discord is open with the vc-userDevTools plugin enabled.";
type Semver = (u16, u16, u16);
const MIN_CLIENT_VERSION: Semver = (0, 1, 3);
/// The version of the server, independent from [`MIN_CLIENT_VERSION`]
const SERVER_VERSION: Semver = (2, 0, 0);

fn check_client_version(version: Semver) -> Result<()> {
	if version < MIN_CLIENT_VERSION {
		bail!(
			"Client version {version:?} is too old, minimum required is {MIN_CLIENT_VERSION:?}"
		);
	} else if version.0 != MIN_CLIENT_VERSION.0 {
		bail!(
			"Client version {version:?} is incompatible with server. Min client version ^{maj}.{min}.{patch}",
			maj = MIN_CLIENT_VERSION.0,
			min = MIN_CLIENT_VERSION.1,
			patch = MIN_CLIENT_VERSION.2
		);
	}
	Ok(())
}

/// What the receive task hands back to a waiting [`WsServer::send_msg`]:
/// either a parsed frame, or the error from failing to parse it
type PendingResponse = Result<types::from_client::FullMessage>;

impl WsServer {
	pub async fn send_msg<T: MsgToClient>(
		&self,
		msg: T,
	) -> Result<T::Response> {
		let notification = T::Response::notification_type();
		let (nonce, rx) = {
			// held across the whole process so we don't 
			// get a nonce from once connection and use it with another
			let inner = self.0.read().await;
			let nonce = inner.mint_nonce();
			let wire_str = serde_json::to_string(&msg.to_wire(nonce))
				.context("Failed to serialize WS message")?;
			let Some(tx) = inner.tx.as_ref() else {
				bail!(NO_CONN_MSG);
			};
			// we don't care about notifications so they don't need a response slot
			let rx = notification.is_none().then(|| {
				let (res_tx, res_rx) = oneshot::channel();
				inner.pending_tx.insert(nonce, res_tx);
				res_rx
			});
			if let Err(e) = tx.send(wire_str) {
				inner.pending_tx.remove(&nonce);
				bail!("Failed to post WS message to outbound channel: {e:?}");
			}
			(nonce, rx)
		};
		let Some(rx) = rx else {
			return Ok(notification
				.expect("no receiver is registered only for notifications"));
		};
		// FIXME: customizable timeout duration
		match timeout(Duration::from_mins(1), rx).await {
			Ok(Ok(res)) => res.and_then(|msg| {
				T::Response::from_wire(msg)
					.context("Failed to extract message from wire")
			}),
			// the sender was dropped, nothing left to clean up
			Ok(Err(e)) => {
				bail!("Failed to receive WS response: {e:?}");
			}
			Err(_) => {
				self.0
					.read()
					.await
					.pending_tx
					.remove(&nonce);
				bail!("Timed out waiting for WS response");
			}
		}
	}
}

mod types;

struct Inner {
	/// The pending outbound messages, stringified JSON
	tx: Option<mpsc::UnboundedSender<String>>,
	pending_tx: DashMap<u32, oneshot::Sender<PendingResponse>>,
	next_nonce: AtomicU32,
	tasks: Option<[JoinHandle<()>; 2]>,
}

impl Drop for Inner {
	fn drop(&mut self) {
		if let Some(tasks) = self.tasks.take() {
			for task in tasks {
				task.abort();
			}
		}
	}
}

/// Allowed origins for ws connections
///
/// we only get connections from plugins running in discord, disallow anything else
const ALLOWED_ORIGINS: &[&str] = &[
	"https://discord.com",
	"https://canary.discord.com",
	"https://ptb.discord.com",
];

// expected signature
#[expect(clippy::result_large_err)]
fn check_valid_origin(
	req: &Request,
	res: Response,
) -> Result<Response, ErrorResponse> {
	let Some(origin) = req.headers().get("Origin") else {
		// no origin header -> native client -> allow
		return Ok(res);
	};
	let origin = origin.to_str().unwrap_or_default();
	if ALLOWED_ORIGINS.contains(&origin) {
		Ok(res)
	} else {
		warn!(%origin, "Rejecting WS connection from invalid origin");
		let mut err =
			ErrorResponse::new(Some(String::from("Forbidden Origin")));
		*err.status_mut() = StatusCode::FORBIDDEN;
		Err(err)
	}
}

impl Inner {
	fn mint_nonce(&self) -> u32 {
		self.next_nonce
			.fetch_add(1, Ordering::Relaxed)
			.checked_add(1)
			.expect("wss nonce overflow")
	}
	fn has_active_conn(&self) -> bool {
		self.tx
			.as_ref()
			.is_some_and(|tx| !tx.is_closed())
	}
	/// Handle an incoming connection
	///
	/// if the connection is accepted, it will be stored in `this`
	async fn handle_conn(
		this: Arc<RwLock<Self>>,
		stream: TcpStream,
		from: SocketAddr,
	) {
		let ws = match accept_hdr_async(stream, check_valid_origin).await {
			Ok(guh) => guh,
			Err(e) => {
				warn!(%from, "Failed to accept WS connection: {e:?}");
				return;
			}
		};
		let (mut sink, mut stream) = ws.split();
		let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel();
		{
			let mut inner = this.write().await;
			if inner.has_active_conn() {
				warn!(%from, "Rejecting new WS connection, already have an active connection");
				return;
			}
			*inner = Self {
				tx: Some(outbound_tx),
				pending_tx: DashMap::new(),
				next_nonce: AtomicU32::new(0),
				tasks: None,
			};
		};
		// send messages from the server to the client
		let this2 = this.clone();
		let send_handle = tokio::spawn(async move {
			let this = this2;
			loop {
				if let Some(msg) = outbound_rx.recv().await {
					trace!(bytes = msg.len(), "Sending WS Message");
					if let Err(e) = sink.send(msg.into()).await {
						warn!(
							"Failed to send WS message, closing connection: {e:?}"
						);
						break;
					}
				} else {
					warn!("Outbound channel closed, closing connection");
					break;
				}
			}
			this.write().await.disconnect();
		});

		let this2 = this.clone();
		let recv_handle = tokio::spawn(async move {
			let this = this2;
			loop {
				match stream.next().await {
					Some(Ok(Message::Text(txt))) => {
						Self::dispatch_text_frame(&this, &txt).await;
					}
					Some(Ok(Message::Binary(bin))) => {
						if let Ok(txt) = str::from_utf8(&bin) {
							Self::dispatch_text_frame(&this, txt).await;
						} else {
							warn!("Received non-UTF8 binary frame, skipping");
						}
					}
					Some(Ok(Message::Close(_))) => {
						info!("WS connection closed by client");
						break;
					}
					Some(Ok(
						Message::Ping(_) | Message::Pong(_) | Message::Frame(_),
					)) => {
						warn!(
							"got unexpected ping/pong/frame message, ignoring"
						);
					}
					Some(Err(e)) => {
						warn!("WS Stream error, closing connection: {e:?}");
						break;
					}
					None => {
						info!(
							"WS backing TCP stream closed, closing conneciton"
						);
						break;
					}
				}
			}
			this.write().await.disconnect();
		});
		this.write().await.tasks = Some([send_handle, recv_handle]);
		Self::handshake(WsServer(this)).await;
	}

	/// Exchange versions with a freshly connected client, dropping the
	/// connection if it is one we can't talk to
	async fn handshake(srv: WsServer) {
		let version_check = try {
			let client_ver = match srv
				.send_msg(types::to_client::VersionMessage {
					server_version: SERVER_VERSION,
				})
				.await
			{
				Ok(v) => v,
				Err(e) => Err(anyhow!("Failed to get client version: {e:?}"))?,
			};
			let client_ver = client_ver.client_version;
			check_client_version(client_ver)?;
		};
		if let Err(e) = version_check {
			error!("Client version check failed, closing connection: {e:?}");
			let WsServer(this) = srv;
			this.write().await.disconnect();
		}
	}

	async fn dispatch_text_frame(this: &RwLock<Self>, txt: &str) {
		/// Just enough of a frame to route one we can't otherwise parse
		#[derive(Deserialize)]
		struct NonceOnly {
			nonce: u32,
		}

		let msg: PendingResponse =
			serde_json::from_str(txt).context("Failed to parse WS message");
		// an unparseable frame still has to reach its waiter, otherwise the
		// caller sits out the whole response timeout for an error we already
		// have in hand
		let nonce = match &msg {
			Ok(msg) => msg.nonce(),
			Err(e) => {
				let Ok(NonceOnly { nonce }) =
					serde_json::from_str::<NonceOnly>(txt)
				else {
					warn!("Failed to parse WS message, ignoring: {e:?}");
					return;
				};
				nonce
			}
		};
		trace!(?msg, "Received WS message");
		let value = this
			.read()
			.await
			.pending_tx
			.remove(&nonce);
		if let Some((_, tx)) = value {
			if tx.send(msg).is_err() {
				warn!(
					"Failed to send WS response to waiting task; receiver dropped"
				);
			} else {
				debug!(%nonce, "Dispatched WS message to waiting task");
			}
		} else {
			warn!(%nonce, "Received WS message with unknown nonce, ignoring");
		}
	}

	fn disconnect(&mut self) {
		*self = Self::disconnected();
	}

	fn disconnected() -> Self {
		Self {
			tx: None,
			pending_tx: DashMap::new(),
			next_nonce: AtomicU32::new(0),
			tasks: None,
		}
	}
}

impl WsServer {
	pub const PORT: u16 = 8484;

	pub fn disconnected() -> Self {
		Self(Arc::new(RwLock::new(Inner::disconnected())))
	}

	pub async fn run_loop(Self(state): Self) -> Result<()> {
		let addr = SocketAddr::from(([127, 0, 0, 1], Self::PORT));
		let listener = TcpListener::bind(addr)
			.await
			.with_context(|| format!("Failed to bind to {addr}"))?;
		info!(%addr, "WS server listening");
		loop {
			let (stream, from) = match listener.accept().await {
				Ok(e) => e,
				Err(e) => {
					warn!("Failed to accept connection, retrying: {e:?}");
					continue;
				}
			};
			tokio::spawn(Inner::handle_conn(state.clone(), stream, from));
		}
	}
}
