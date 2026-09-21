//! Diagnostics for the patches of a vencord plugin.

use std::{
	collections::HashMap,
	future::pending,
	hash::Hash as _,
	path::Path,
	sync::{Arc, mpsc as sync_mpsc},
};

use ast_parser::pool::AllocPool;
use futures_util::future::join_all;
use parser_diag::{LocalSource, ParserDiagnostic};
use tokio::{
	sync::{broadcast, mpsc},
	time::{Duration, Instant, sleep_until},
};
use tower_lsp_server::{
	Client,
	ls_types::{
		Diagnostic,
		DiagnosticRelatedInformation,
		DiagnosticSeverity,
		Location,
		NumberOrString,
		Range,
		Uri,
	},
};
use tracing::{debug, instrument, trace, warn};
use vencord_ast_parser::{FindArg, VencordAstParser};
use xxhash_rust::xxh64::Xxh64;

use crate::{
	lsp::{
		self,
		doc::{Document, Files},
		lenses::is_plugin_path,
		test_patch::patch_to_wire,
	},
	util::{err::is_caused_by, uri},
	wss::{
		NoClientsError,
		WsServer,
		types::{
			from_client::ClientError,
			to_client::{
				FindData,
				FindNode,
				RegexValue,
				TestFindMessage,
				TextPatchMessage,
			},
		},
	},
};

/// The `source` every diagnostic we push is tagged with.
const SOURCE: &str = "vencord-companion";

/// How long a file has to go unedited before it is linted again.
///
/// Parsing is not incremental, so a keystroke would otherwise re-parse the
/// whole file.
const DEBOUNCE: Duration = Duration::from_millis(300);

const fn severity(severity: miette::Severity) -> DiagnosticSeverity {
	match severity {
		miette::Severity::Advice => DiagnosticSeverity::INFORMATION,
		miette::Severity::Warning => DiagnosticSeverity::WARNING,
		miette::Severity::Error => DiagnosticSeverity::ERROR,
	}
}

/// Converts a [`ParserDiagnostic`] into the LSP diagnostic for `doc`.
fn to_lsp(diag: &ParserDiagnostic, doc: &Document, uri: &Uri) -> Diagnostic {
	let range = diag
		.labels
		.first()
		.map_or_else(Range::default, |&(span, _)| doc.range_for_span(span));
	let related: Vec<_> = diag
		.labels
		.iter()
		.filter(|(_, label)| !label.is_empty())
		.map(|&(span, ref label)| DiagnosticRelatedInformation {
			location: Location {
				uri: uri.clone(),
				range: doc.range_for_span(span),
			},
			message: label.clone().into_owned(),
		})
		.collect();
	let message = match &diag.cause {
		// the cause is an opaque `miette::Diagnostic` with spans of its own,
		// so it can only be rendered into the message
		Some(cause) => format!("{}\n\ncaused by: {cause}", diag.msg),
		None => diag.msg.clone().into_owned(),
	};
	Diagnostic {
		range,
		severity: Some(severity(diag.severity)),
		source: Some(String::from(SOURCE)),
		message,
		related_information: (!related.is_empty()).then_some(related),
		..Diagnostic::default()
	}
}

/// The deepest message in `e`'s source chain.
///
/// That is the one the client actually sent; everything above it is our own
/// plumbing, which is no help to whoever is looking at the squiggle.
fn root_message(e: &anyhow::Error) -> String {
	e.chain()
		.last()
		.map_or_else(|| e.to_string(), ToString::to_string)
}

/// A find argument, as the client wants it.
fn find_arg_to_wire(arg: &FindArg) -> FindNode {
	match arg {
		FindArg::String(value) => FindNode::String {
			value: value.clone(),
		},
		FindArg::Regex { pattern, flags } => FindNode::Regex {
			value: RegexValue {
				pattern: pattern.clone(),
				flags: flags.clone(),
			},
		},
		// a find function has no value we could send on its own, so it goes
		// over the wire as its source text for the client to `eval`
		FindArg::Function(body) => FindNode::Function { body: body.clone() },
	}
}

/// `find` as the `testFind` payload.
fn find_to_wire(find: &vencord_ast_parser::FindData) -> TestFindMessage {
	TestFindMessage {
		find: FindData {
			find_type: find.kind.to_string(),
			args: find
				.args
				.iter()
				.map(find_arg_to_wire)
				.collect(),
		},
	}
}

/// Something in a plugin file that only the running client can tell us is
/// working.
///
/// we *could* tell if it's broken by running the patch/find outselves, but we can't run every one
#[derive(Hash)]
enum Probe {
	/// A patch, to be applied to the module its find matches.
	Patch(TextPatchMessage),
	/// A webpack find, to be run against the loaded modules.
	Find(TestFindMessage),
}

impl Probe {
	/// The `code` the diagnostic for a failed probe is tagged with.
	const fn code(&self) -> &'static str {
		match self {
			Self::Patch(_) => "patch",
			Self::Find(_) => "find",
		}
	}

	/// A hash of what would go over the wire.
	///
	/// Two probes with the same hash ask the client the same question, so
	/// the answer to one is the answer to the other. The source span is
	/// deliberately not part of it: a patch that only moved down the file
	/// still matches the same module.
	fn wire_hash(&self) -> u64 {
		let mut h = Xxh64::default();
		self.hash(&mut h);
		h.digest()
	}
}

/// The cache key of the probe for `msg`.
///
/// Goes through [`Probe`] so it can't drift from the key the linter stores
/// its answers under.
pub(in crate::lsp) fn patch_probe_hash(msg: &TextPatchMessage) -> u64 {
	Probe::Patch(msg.clone()).wire_hash()
}

/// A [`Probe`], the range in the document it came from, and its
/// [`Probe::wire_hash`].
struct Pending {
	range: Range,
	hash: u64,
	probe: Probe,
}

impl Pending {
	fn new(range: Range, probe: Probe) -> Self {
		Self {
			range,
			hash: probe.wire_hash(),
			probe,
		}
	}
}

/// What the client said about a probe, kept so the same question isn't asked
/// twice.
#[derive(Clone)]
struct Answer {
	/// The `code` of the probe this answers, for the diagnostic.
	code: &'static str,
	/// The client's complaint, or `None` if it was happy.
	message: Option<String>,
}

/// Every patch and find in `parser`, as the messages that ask the client to
/// try each one.
///
/// A file that has no patches or no finds is the common case, not an error,
/// so a failure to parse either one only drops that half of the probes.
fn probes(parser: &VencordAstParser, doc: &Document) -> Vec<Pending> {
	let mut ret = Vec::new();
	match parser.patches(true) {
		Ok(patches) => {
			for patch in patches {
				match patch_to_wire(&patch, doc.text.as_str()) {
					Ok(msg) => ret.push(Pending::new(
						doc.range_for_span(patch.span),
						Probe::Patch(msg),
					)),
					Err(e) => debug!(
						"Failed to build testPatch payload, skipping: {e:?}"
					),
				}
			}
		}
		Err(e) => {
			debug!("Failed to parse patches, skipping patch probes: {e:?}");
		}
	}
	match parser.get_finds() {
		Ok(finds) => ret.extend(finds.iter().map(|find| {
			Pending::new(
				doc.range_for_span(find.range),
				Probe::Find(find_to_wire(&find.data)),
			)
		})),
		Err(e) => {
			debug!("Failed to parse finds, skipping find probes: {e:?}");
		}
	}
	ret
}

/// What the [`Worker`] has been asked to do with a file.
pub(in crate::lsp) enum Request {
	/// Lint the file once it has gone `delay` without another request.
	Lint { uri: Uri, delay: Duration },
	/// Forget the file and drop the diagnostics pushed for it.
	Clear(Uri),
	/// Forget what the client said about one probe in the file, and lint it
	/// again to ask afresh.
	Invalidate { uri: Uri, hash: u64 },
}

/// Queues plugin files to be linted.
pub struct Diagnostics {
	requests: mpsc::UnboundedSender<Request>,
}

impl Diagnostics {
	pub(in crate::lsp) fn new() -> (Self, mpsc::UnboundedReceiver<Request>) {
		let (requests, rx) = mpsc::unbounded_channel();
		(Self { requests }, rx)
	}

	/// Lints queued files and pushes their diagnostics to the client.
	///
	/// A single task does all the linting, so two publishes for the same file
	/// can never land out of order and leave the client showing stale
	/// diagnostics.
	pub(in crate::lsp) fn serve(
		client: Client,
		files: Files,
		requests: mpsc::UnboundedReceiver<Request>,
		pool: Arc<AllocPool>,
		ws: WsServer,
	) {
		let connected = ws.on_connect();
		tokio::spawn(
			Worker {
				client,
				files,
				pool,
				ws,
				pending: HashMap::new(),
				probe_cache: HashMap::new(),
			}
			.run(requests, connected),
		);
	}

	fn send(&self, request: Request) {
		if self.requests.send(request).is_err() {
			warn!("Dropping diagnostics request: the linter task is gone");
		}
	}

	/// Lints `uri` as soon as the worker gets to it.
	fn lint_now(&self, uri: Uri) {
		self.send(Request::Lint {
			uri,
			delay: Duration::ZERO,
		});
	}

	/// Lints `uri` once it has gone [`DEBOUNCE`] without another edit.
	fn lint_debounced(&self, uri: Uri) {
		self.send(Request::Lint {
			uri,
			delay: DEBOUNCE,
		});
	}

	/// Cancels any queued lint of `uri` and drops its diagnostics.
	fn clear(&self, uri: Uri) {
		self.send(Request::Clear(uri));
	}

	/// Drops the cached answer for `msg` in `uri` and re-lints the file.
	///
	/// For when the user asked the client about that patch themselves: the
	/// answer we are sitting on is no longer the last word.
	pub(in crate::lsp) fn invalidate_patch(
		&self,
		uri: Uri,
		msg: &TextPatchMessage,
	) {
		self.send(Request::Invalidate {
			uri,
			hash: patch_probe_hash(msg),
		});
	}
}

struct Worker {
	client: Client,
	files: Files,
	pool: Arc<AllocPool>,
	ws: WsServer,
	/// Files waiting to be linted, and the instant each one is due.
	pending: HashMap<Uri, Instant>,
	/// What the client last said about each probe, per file, keyed by
	/// [`Probe::wire_hash`].
	///
	/// Only answers the client itself gave are in here, and they only hold
	/// for the connection that gave them.
	probe_cache: HashMap<Uri, HashMap<u64, Answer>>,
}

impl Worker {
	async fn run(
		mut self,
		mut requests: mpsc::UnboundedReceiver<Request>,
		mut connected: broadcast::Receiver<()>,
	) {
		loop {
			// with nothing pending there is no file to come due, so this
			// arm just never fires
			let due = self.next_due();
			let due = async move {
				match due {
					Some(due) => sleep_until(due).await,
					None => pending().await,
				}
			};
			let request = tokio::select! {
				// a request that arrives right as a file comes due
				// pushes that file back, which is the whole point of
				// the debounce
				biased;
				request = requests.recv() => request,
				event = connected.recv() => {
					match event {
						// a fresh client can answer for the patches and
						// finds we had to leave unchecked
						Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
							self.relint_open();
						}
						// we hold a `WsServer` ourselves, so its sender
						// outlives us and this is unreachable
						Err(broadcast::error::RecvError::Closed) => {
							warn!("ws server is gone, stopping the linter");
							break;
						}
					}
					continue;
				}
				() = due => {
					self.lint_due().await;
					continue;
				}
			};
			// every sender is gone, so the server is shutting down
			let Some(request) = request else { break };
			match request {
				Request::Lint { uri, delay } => {
					self.pending
						.insert(uri, Instant::now() + delay);
				}
				Request::Clear(uri) => {
					self.pending.remove(&uri);
					self.probe_cache.remove(&uri);
					self.publish(&uri, Vec::new(), None)
						.await;
				}
				Request::Invalidate { uri, hash } => {
					if let Some(cache) = self.probe_cache.get_mut(&uri) {
						cache.remove(&hash);
					}
					// a file mid-edit keeps its debounce, so this never cuts
					// an edit short
					self.pending
						.entry(uri)
						.or_insert_with(Instant::now);
				}
			}
		}
	}

	/// Queues every open file, for when a client turns up and the probes that
	/// were skipped can finally be answered.
	///
	/// A file already waiting on its debounce keeps that deadline, so this
	/// never cuts an edit short.
	fn relint_open(&mut self) {
		// a new client has its own modules loaded, so nothing the last one
		// said still holds
		self.probe_cache.clear();
		let now = Instant::now();
		for uri in self.files.open_uris() {
			self.pending.entry(uri).or_insert(now);
		}
		debug!(count = self.pending.len(), "client connected, re-linting");
	}

	/// The instant the first pending file comes due, if any is pending.
	fn next_due(&self) -> Option<Instant> {
		self.pending.values().copied().min()
	}

	/// Lints every file whose debounce has run out.
	async fn lint_due(&mut self) {
		let now = Instant::now();
		let due: Vec<Uri> = self
			.pending
			.iter()
			.filter(|&(_, &deadline)| deadline <= now)
			.map(|(uri, _)| uri.clone())
			.collect();
		for uri in due {
			self.pending.remove(&uri);
			self.lint(&uri).await;
		}
	}

	/// Lints `uri` as a vencord plugin and pushes the result to the client.
	///
	/// Files that aren't plugins are skipped entirely, and a file that fails
	/// to parse clears its diagnostics instead of reporting the parse error;
	/// the client's own TypeScript tooling already reports those.
	#[instrument(skip_all, fields(uri =% uri.as_str()))]
	async fn lint(&mut self, uri: &Uri) {
		let path = match uri::to_path(uri) {
			Ok(p) => p,
			Err(e) => {
				trace!("uri is not a file path, skipping diagnostics: {e}");
				return;
			}
		};
		if !is_plugin_path(&path) {
			trace!(?path, "not a plugin file, skipping diagnostics");
			return;
		}
		let Some(doc) = self.files.get(uri) else {
			debug!("no document found for uri, skipping diagnostics");
			return;
		};
		// the parser is neither `Send` nor `Sync`, so it has to be gone
		// before anything below is awaited
		let (mut diagnostics, probes) = self.collect(uri, &doc, &path);
		// without a client there is nothing to try the patches and finds
		// against, so they are left unreported rather than flagged as broken
		if !probes.is_empty() && self.ws.has_connection().await {
			diagnostics.extend(self.run_probes(uri, probes).await);
		}
		debug!(count = diagnostics.len(), "publishing plugin diagnostics");
		self.publish(uri, diagnostics, Some(doc.version()))
			.await;
	}

	/// Parses `uri` once, returning the diagnostics the parser found on its
	/// own and the probes that need a running client to answer.
	fn collect(
		&self,
		uri: &Uri,
		doc: &Document,
		path: &Path,
	) -> (Vec<Diagnostic>, Vec<Pending>) {
		let alloc = self.pool.get();
		let path_str = path.to_string_lossy();
		let mut parser =
			match VencordAstParser::try_new(&alloc, &doc.text, Some(&path_str))
			{
				Ok(parser) => parser,
				Err(inner) => {
					let e = LocalSource {
						inner,
						name: &path_str,
						source: &doc.text,
					};
					debug!(
						"Failed to parse plugin file, skipping diagnostics:{e:?}"
					);
					return (Vec::new(), Vec::new());
				}
			};
		let (tx, rx) = sync_mpsc::channel();
		parser.diag_ch = Some(tx);
		parser.collect_diagnostics();
		// drop the only sender, so draining the receiver terminates
		parser.diag_ch = None;
		let diagnostics = rx
			.into_iter()
			.map(|diag| to_lsp(&diag, doc, uri))
			.collect();
		(diagnostics, probes(&parser, doc))
	}

	/// Sends the probes this file hasn't already had an answer to, and turns
	/// the refusals into diagnostics.
	///
	/// The answers are kept keyed by [`Probe::wire_hash`], so editing one
	/// patch only re-sends that patch, and the rest of the file costs
	/// nothing. Answers for probes that are no longer in the file are
	/// dropped, which keeps the cache the size of the file rather than the
	/// size of the editing session.
	async fn run_probes(
		&mut self,
		uri: &Uri,
		probes: Vec<Pending>,
	) -> Vec<Diagnostic> {
		let cached = self
			.probe_cache
			.remove(uri)
			.unwrap_or_default();
		// two probes with the same hash ask the same question, so only the
		// first of them is sent
		let mut unsent: HashMap<u64, Probe> = HashMap::new();
		let mut seen: Vec<(Range, u64)> = Vec::with_capacity(probes.len());
		for Pending { range, hash, probe } in probes {
			seen.push((range, hash));
			if !cached.contains_key(&hash) {
				unsent.entry(hash).or_insert(probe);
			}
		}
		debug!(
			sent = unsent.len(),
			reused = seen.len() - unsent.len(),
			"running probes"
		);
		let ws = &self.ws;
		let results = join_all(unsent.into_iter().map(
			|(hash, probe)| async move {
				let code = probe.code();
				let result = match probe {
					Probe::Patch(msg) => ws.send_msg(msg).await.map(drop),
					Probe::Find(msg) => ws.send_msg(msg).await.map(drop),
				};
				(hash, code, result)
			},
		))
		.await;
		let mut next = HashMap::with_capacity(seen.len());
		let mut fresh = HashMap::new();
		for (hash, code, result) in results {
			let (answer, keep) = match result {
				Ok(()) => (
					Some(Answer {
						code,
						message: None,
					}),
					true,
				),
				// the client went away while we were asking; that isn't a
				// problem with the patch, and flagging every one of them
				// would bury the file in squiggles
				Err(e) if is_caused_by::<NoClientsError>(&*e) => {
					debug!("client disconnected mid-lint, dropping probe");
					(None, false)
				}
				Err(e) => {
					// only the client's own verdict is worth keeping; a
					// timeout or a dropped frame says nothing about the
					// patch, so it gets asked again next time
					let keep = is_caused_by::<ClientError>(&*e);
					let answer = Answer {
						code,
						message: Some(root_message(&e)),
					};
					(Some(answer), keep)
				}
			};
			if let Some(answer) = answer {
				if keep {
					next.insert(hash, answer.clone());
				}
				fresh.insert(hash, answer);
			}
		}
		// carry the reused answers over, dropping any whose probe is gone
		// from the file
		for (_, hash) in &seen {
			if let Some(answer) = cached.get(hash) {
				next.insert(*hash, answer.clone());
			}
		}
		self.probe_cache
			.insert(uri.clone(), next);
		seen.into_iter()
			.filter_map(|(range, hash)| {
				let answer = fresh
					.get(&hash)
					.or_else(|| cached.get(&hash))?;
				Some(Diagnostic {
					range,
					severity: Some(DiagnosticSeverity::ERROR),
					source: Some(String::from(SOURCE)),
					code: Some(NumberOrString::String(String::from(
						answer.code,
					))),
					message: answer.message.clone()?,
					..Diagnostic::default()
				})
			})
			.collect()
	}

	async fn publish(
		&self,
		uri: &Uri,
		diagnostics: Vec<Diagnostic>,
		version: Option<i32>,
	) {
		self.client
			.publish_diagnostics(uri.clone(), diagnostics, version)
			.await;
	}
}

impl lsp::Server {
	/// Queues `uri` to be linted as soon as possible.
	pub(super) fn diagnostics_open_hook(&self, uri: Uri) {
		self.diagnostics.lint_now(uri);
	}

	/// Queues `uri` to be linted once the edits stop coming in.
	pub(super) fn diagnostics_change_hook(&self, uri: Uri) {
		self.diagnostics.lint_debounced(uri);
	}

	/// Drops every diagnostic pushed for `uri`.
	pub(super) fn diagnostics_close_hook(&self, uri: Uri) {
		self.diagnostics.clear(uri);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::wss::types::to_client::{
		MatchNode,
		PatchFind,
		PatchReplacement,
		ReplaceNode,
	};

	fn patch(find: &str, replace: &str) -> Probe {
		Probe::Patch(TextPatchMessage {
			find: PatchFind::String {
				string: String::from(find),
			},
			replace: vec![PatchReplacement {
				match_: MatchNode::String {
					value: String::from("a"),
				},
				replace: ReplaceNode::String {
					value: String::from(replace),
				},
			}],
		})
	}

	fn find(kind: &str) -> Probe {
		Probe::Find(TestFindMessage {
			find: FindData {
				find_type: String::from(kind),
				args: vec![FindNode::String {
					value: String::from("a"),
				}],
			},
		})
	}

	#[test]
	fn wire_hash_tracks_the_payload() {
		assert_eq!(
			patch("x", "y").wire_hash(),
			patch("x", "y").wire_hash(),
			"the same patch has to reuse its answer"
		);
		assert_ne!(
			patch("x", "y").wire_hash(),
			patch("x", "z").wire_hash(),
			"an edited replacement has to be asked again"
		);
		assert_ne!(
			patch("x", "y").wire_hash(),
			patch("w", "y").wire_hash(),
			"an edited find has to be asked again"
		);
	}

	#[test]
	fn wire_hash_separates_the_message_types() {
		assert_eq!(
			find("findByProps").wire_hash(),
			find("findByProps").wire_hash()
		);
		assert_ne!(
			find("findByProps").wire_hash(),
			find("findStore").wire_hash()
		);
		assert_ne!(
			patch("a", "a").wire_hash(),
			find("a").wire_hash(),
			"a patch and a find are different questions"
		);
	}
}
