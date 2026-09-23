use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use const_format::formatc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tower_lsp_server::{
	Client,
	ls_types::{Uri, notification::Notification, request::Request},
};
use tracing::{debug, instrument, warn};

use crate::{LspResult, SERVER_NAME, lsp, util::uri};

#[derive(Serialize, Deserialize, Clone)]
/// A change made to an ephemeral document
pub struct EphemeralChange {
	/// the URI of the document that changed
	pub uri: Uri,
	/// if present, the content of the document has changed
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content: Option<Arc<str>>,
	/// if true, the document has been closed/"deleted"
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deleted: Option<bool>,
}

impl From<EphemeralDocument> for EphemeralChange {
	fn from(EphemeralDocument { uri, content }: EphemeralDocument) -> Self {
		Self {
			uri,
			content: Some(content),
			deleted: None,
		}
	}
}

impl Notification for EphemeralChange {
	const METHOD: &'static str = formatc!("$/{SERVER_NAME}/ephemera/didChange");
	type Params = Self;
}

#[derive(Serialize, Deserialize)]
pub struct EphemeralQuery {
	uri: Uri,
}

#[derive(Serialize, Deserialize)]
pub struct EphemeralQueryResult {
	pub doc: Option<EphemeralDocument>,
}

impl Request for EphemeralQuery {
	type Params = Self;

	type Result = EphemeralQueryResult;

	const METHOD: &'static str = formatc!("$/{SERVER_NAME}/ephemera/queryDoc");
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EphemeralDocument {
	pub uri: Uri,
	pub content: Arc<str>,
}

pub struct Ephemera {
	documents: DashMap<Box<str>, EphemeralDocument>,
	changes: mpsc::UnboundedSender<EphemeralChange>,
}

impl Ephemera {
	pub(in crate::lsp) fn new()
	-> (Self, mpsc::UnboundedReceiver<EphemeralChange>) {
		let (changes, rx) = mpsc::unbounded_channel();
		(
			Self {
				documents: DashMap::new(),
				changes,
			},
			rx,
		)
	}

	/// Forwards queued changes to the client, one at a time, in the order they
	/// were queued.
	///
	/// A task spawned per notification would let two changes to the same
	/// document be delivered out of order, leaving the client's copy
	/// permanently stale.
	pub(in crate::lsp) fn serve_changes(
		client: Client,
		mut changes: mpsc::UnboundedReceiver<EphemeralChange>,
	) {
		tokio::spawn(async move {
			while let Some(change) = changes.recv().await {
				client
					.send_notification::<EphemeralChange>(change)
					.await;
			}
		});
	}

	fn emit(&self, change: EphemeralChange) {
		if self.changes.send(change).is_err() {
			warn!("Dropping ephemeral change: the notifier task is gone");
		}
	}

	/// The document stored under `uri`, if there is one.
	pub fn get(&self, uri: &Uri) -> Option<EphemeralDocument> {
		self.documents
			.get(&key(uri))
			.map(|d| d.clone())
	}

	/// Store `doc`, warning if it replaces one.
	pub fn create(&self, doc: EphemeralDocument) {
		if self
			.documents
			.insert(key(&doc.uri), doc.clone())
			.is_some()
		{
			warn!(uri =% doc.uri.as_str(), "Overwriting ephemeral document");
		}
		self.emit(doc.into());
	}

	/// Store `doc`, replacing any document already under its URI.
	///
	/// Unlike [`Self::create`], replacing is the expected case: a caller that
	/// re-publishes a document it owns, such as a live module being re-fetched
	/// after it was invalidated, has not lost track of anything.
	pub fn upsert(&self, doc: EphemeralDocument) {
		self.documents
			.insert(key(&doc.uri), doc.clone());
		self.emit(doc.into());
	}

	#[instrument(skip_all, fields(uri =% doc.uri.as_str()))]
	pub fn update(&self, doc: EphemeralChange) -> Result<()> {
		if doc.deleted == Some(true) {
			debug!("Deleting ephemeral document");
			self.delete(doc.uri)
				.context("Failed to delete ephemeral document")?;
		} else {
			let mut our_doc = self
				.documents
				.get_mut(&key(&doc.uri))
				.context("No such ephemeral document")?;
			if let Some(new_content) = &doc.content {
				our_doc.content = new_content.clone();
			}
			drop(our_doc);
			self.emit(doc);
		}
		Ok(())
	}

	#[instrument(skip_all, fields(uri =% uri.as_str()))]
	pub fn delete(&self, uri: Uri) -> Result<()> {
		#[rustfmt::skip]
		ensure!(self.documents.remove(&key(&uri)).is_some());
		debug!("Deleting ephemeral document");
		self.emit(EphemeralChange {
			uri,
			content: None,
			deleted: Some(true),
		});
		Ok(())
	}
}

const SCHEME: &str = SERVER_NAME;

/// The key a document is stored under.
///
/// Not the raw URI string: the client parses and re-serializes our URIs with
/// its own percent-encoding rules, which encode a different set of characters
/// than [`uri::ASCII_SET`] does, and drop an empty authority. The decoded path
/// survives that round trip unchanged.
fn key(uri: &Uri) -> Box<str> {
	uri.path()
		.decode()
		.to_string_lossy()
		.into()
}

pub fn uri<A: AsRef<str>>(path: A) -> Result<Uri> {
	uri::from_path_with_scheme(SCHEME, path)
}

// TODO: add tests for server state
impl lsp::Server {
	pub fn query_ephemeral_document(
		&self,
		params: &EphemeralQuery,
	) -> LspResult<EphemeralQueryResult> {
		Ok(EphemeralQueryResult {
			doc: self.ephemera.get(&params.uri),
		})
	}

	pub fn create_ephemeral_document(&self, doc: EphemeralDocument) {
		self.ephemera.create(doc);
	}

	pub fn update_ephemeral_document(
		&self,
		doc: EphemeralChange,
	) -> Result<()> {
		self.ephemera.update(doc)
	}

	pub fn delete_ephemeral_document(&self, uri: Uri) -> Result<()> {
		self.ephemera.delete(uri)
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr as _;

	use tower_lsp_server::ls_types::Uri;

	use super::{key, uri};

	#[test]
	fn uris_round_trip_through_the_client() {
		let doc = uri("/test-ephemeral-doc.txt").unwrap();
		assert_eq!(doc.as_str(), "vencord-companion:/test-ephemeral-doc.txt");

		let forms = [
			// vscode.Uri(..).toString()
			"vencord-companion:/test-ephemeral-doc.txt",
			"vencord-companion:///test-ephemeral-doc.txt",
		];
		for form in forms {
			let parsed = Uri::from_str(form).unwrap();
			assert_eq!(key(&parsed), key(&doc), "{form}");
		}
	}

	/// VS Code leaves `(` and `)` unencoded in a path; [`uri::ASCII_SET`]
	/// encodes them.
	#[test]
	fn keys_ignore_percent_encoding_differences() {
		let doc = uri("/a(b).txt").unwrap();
		let from_client = Uri::from_str("vencord-companion:/a(b).txt").unwrap();
		assert_eq!(key(&from_client), key(&doc));
	}
}
