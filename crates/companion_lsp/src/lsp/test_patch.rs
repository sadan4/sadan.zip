//! The `test_patch` command: sends a patch to the connected client and asks it
//! to apply the patch to the module its find matches.

use anyhow::{Context as _, Result};
use oxc::span::Span;
use tower_lsp_server::ls_types::{MessageType, Uri};
use tracing::{info, warn};
use vencord_ast_parser::{Match, Patch, ReplaceLike, Replacer};

use crate::{
	lsp::lenses::PatchLensArgs,
	util::err::is_caused_by,
	wss::{
		NoClientsError,
		types::to_client::{
			MatchNode,
			PatchFind,
			PatchReplacement,
			RegexValue,
			ReplaceNode,
			TextPatchMessage,
		},
	},
};

/// The `find` of a patch, as the client wants it: a bare string, or a
/// stringified regex literal.
fn find_to_wire(find: &Match) -> PatchFind {
	match find {
		Match::Str(f) => PatchFind::String {
			string: str::from_utf8(f.needle())
				.expect("always utf8")
				.to_owned(),
		},
		Match::Regex(r) => PatchFind::Regex {
			regex: format!("/{}/{}", r.pattern, r.flags),
		},
	}
}

/// The `match` of a replacement, as a `StringNode | RegexNode`.
fn match_to_wire(match_: &Match) -> MatchNode {
	match match_ {
		Match::Str(f) => MatchNode::String {
			value: str::from_utf8(f.needle())
				.expect("always utf8")
				.to_owned(),
		},
		Match::Regex(r) => MatchNode::Regex {
			value: RegexValue {
				pattern: r.pattern.clone(),
				flags: r.flags.to_string(),
			},
		},
	}
}

/// The source text `span` covers in `src`.
fn source_text(src: &str, span: Span) -> Result<&str> {
	src.get(span.start as usize..span.end as usize)
		.context("replace span is not in the document")
}

/// The `replace` of a replacement, as a `StringNode | FunctionNode`.
///
/// A replace function has no value we could send on its own, so it goes over
/// the wire as its source text for the client to `eval`, which is what the
/// typescript extension does.
fn replace_to_wire(replace: &ReplaceLike, src: &str) -> Result<ReplaceNode> {
	Ok(match &replace.v {
		Replacer::Str(s) => ReplaceNode::String { value: s.clone() },
		Replacer::Template(_) => ReplaceNode::Function {
			value: source_text(src, replace.s)?.to_owned(),
		},
	})
}

/// `patch` as the `testPatch` payload, taking the text of replace functions
/// out of `src`.
fn patch_to_wire(patch: &Patch, src: &str) -> Result<TextPatchMessage> {
	Ok(TextPatchMessage {
		find: find_to_wire(&patch.find.v),
		replace: patch
			.replacement
			.iter()
			.map(|r| {
				Ok(PatchReplacement {
					match_: match_to_wire(&r.match_.v),
					replace: replace_to_wire(&r.replace, src)?,
				})
			})
			.collect::<Result<_>>()?,
	})
}

impl super::Server {
	/// Builds the `testPatch` payload for the patch in `uri` hashing to
	/// `hash`.
	///
	/// The document is looked up after the patch is parsed out of it, so the
	/// two reads never overlap.
	fn test_patch_message(
		&self,
		uri: &Uri,
		hash: u64,
	) -> Result<TextPatchMessage> {
		let patch = self.patch_for_hash(uri, hash)?;
		let doc = self
			.files
			.get(uri)
			.context("document not found")?;
		patch_to_wire(&patch, doc.text.as_str())
	}

	pub(super) async fn test_patch(&self, args: PatchLensArgs) -> Result<()> {
		let msg = self.test_patch_message(&args.uri, args.hash)?;
		match self.state.ws.send_msg(msg).await {
			Ok(_) => {
				info!("patch applied cleanly");
				self.client
					.show_message(MessageType::INFO, "Patch OK!")
					.await;
				Ok(())
			}
			// let the caller turn this into the usual "no clients" message
			Err(e) if is_caused_by::<NoClientsError>(&*e) => Err(e),
			Err(e) => {
				warn!("Patch failed: {e:?}");
				self.client
					.show_message(
						MessageType::ERROR,
						format!("Patch failed: {e}"),
					)
					.await;
				Ok(())
			}
		}
	}
}
