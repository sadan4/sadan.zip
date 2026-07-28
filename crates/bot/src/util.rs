use std::{path::PathBuf, str::FromStr};

use anyhow::{Context as _, Result, anyhow, bail};
use derive_more::{Deref, Display, From, Into};
use serenity::all::{Message, UserId};
use smol_str::SmolStr;
use tokio::{fs, task_local};

#[derive(
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Hash,
	PartialOrd,
	Ord,
	From,
	Into,
	Deref,
	Display,
)]
pub struct UserArg(pub UserId);

const USER_MENTION_START: &str = "<@";
const USER_MENTION_END: &str = ">";

/// Sentinel `default_value` for a `UserArg`: when the user omits the argument,
/// clap substitutes this string, and `FromStr` resolves it from
/// `REFERENCED_USER`.
pub const FROM_REPLY: &str = "\u{0}FROM_REPLY";

task_local! {
	pub static REFERENCED_USER: Option<UserId>;
	pub static MESSAGE_RECEIVE_TIME: std::time::Instant;
}

impl UserArg {
	pub fn from_mention(mention: &str) -> Result<Self> {
		if mention.starts_with(USER_MENTION_START)
			&& mention.ends_with(USER_MENTION_END)
		{
			let id: u64 = mention[USER_MENTION_START.len()
				..mention.len() - USER_MENTION_END.len()]
				.parse()?;
			Ok(Self(UserId::new(id)))
		} else {
			Err(anyhow!("not a mention"))
		}
	}
}

impl FromStr for UserArg {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		if s == FROM_REPLY {
			match REFERENCED_USER.try_with(|u| *u) {
				Ok(Some(id)) => Ok(Self(id)),
				Ok(None) | Err(_) => {
					bail!("no user given and no referenced message")
				}
			}
		} else if let Ok(user) = Self::from_mention(s) {
			Ok(user)
		} else if let Ok(id) = s.parse() {
			Ok(Self(UserId::new(id)))
		} else {
			bail!("not a mention or user id, nor was there a referenced user");
		}
	}
}

pub fn get_ref_user(msg: &Message) -> Option<UserId> {
	Some(
		msg.referenced_message
			.as_ref()?
			.author
			.id,
	)
}

/// Discord's maximum message length, in characters.
const MAX_MESSAGE_LEN: usize = 2000;

/// Wrap `text` in a Discord code block tagged with `lang` (e.g. `"ansi"`, or
/// `""` for an untagged block), truncating the content when necessary so the
/// whole message stays within Discord's 2000-character limit.
///
/// Only an `ansi`-tagged block renders escape sequences, so ANSI-aware
/// handling is applied only then: the cut never lands inside a sequence (see
/// [`ansi_truncation_point`]) and an ANSI reset precedes the ellipsis so no
/// open style leaks past it. Any other `lang` is truncated on a plain
/// codepoint boundary with a bare ellipsis, since an escape byte would render
/// literally there.
pub fn wrap_code_block(text: &str, lang: &str) -> String {
	const FOOTER: &str = "\n```";
	debug_assert!(lang.is_ascii(), "Discord code block tags must be ASCII");
	let header = format!("```{lang}\n");
	let max_body = MAX_MESSAGE_LEN - header.len() - FOOTER.len();
	let is_ansi = lang == "ansi";
	// an ANSI reset closes any style left open by the cut; in a non-ansi block
	// it would render as literal garbage, so use a bare ellipsis there
	let ellipsis = if is_ansi { "\u{1b}[0m…" } else { "…" };
	let mut body = text.to_owned();
	if body.chars().count() > max_body {
		let budget = max_body - ellipsis.chars().count();
		let cut = if is_ansi {
			ansi_truncation_point(&body, budget)
		} else {
			codepoint_truncation_point(&body, budget)
		};
		body.truncate(cut);
		body.push_str(ellipsis);
	}
	let mut out = String::with_capacity(
		MAX_MESSAGE_LEN.min(header.len() + body.len() + FOOTER.len()),
	);
	out.push_str(&header);
	out.push_str(&body);
	out.push_str(FOOTER);
	out
}

/// Returns the byte index at which `s` can be truncated so that what remains
/// is at most `max` codepoints long.
fn codepoint_truncation_point(s: &str, max: usize) -> usize {
	s.char_indices()
		.nth(max)
		.map_or(s.len(), |(idx, _)| idx)
}

/// Returns the byte index at which `s` can be truncated so that what remains
/// is at most `max` codepoints long.
///
/// A cut that would land inside an ANSI escape sequence is moved back to the
/// start of that sequence, so the tail of a sequence is never left behind to
/// be rendered as literal text.
pub fn ansi_truncation_point(s: &str, max: usize) -> usize {
	/// Where in an escape sequence the scanner currently is.
	enum State {
		Text,
		/// An escape has been seen, but not the byte that says what kind of
		/// sequence it introduces.
		Escape,
		/// Inside a control sequence, waiting on its final byte.
		Csi,
	}
	let mut state = State::Text;
	// the start of the escape sequence being scanned, if any
	let mut escape_start = 0;
	for (count, (idx, ch)) in s.char_indices().enumerate() {
		if count == max {
			return match state {
				State::Text => idx,
				State::Escape | State::Csi => escape_start,
			};
		}
		state = match state {
			State::Text if ch == '\u{1b}' => {
				escape_start = idx;
				State::Escape
			}
			State::Escape if ch == '[' => State::Csi,
			// a control sequence ends on a final byte in this range;
			// everything before it is a parameter or intermediate byte
			State::Csi if matches!(ch, '\u{40}'..='\u{7e}') => State::Text,
			State::Csi => State::Csi,
			// anything other than a `[` after the escape is a two character
			// sequence, which this character terminates
			State::Text | State::Escape => State::Text,
		};
	}
	s.len()
}

pub async fn mktemp(prefix: &str, suffix: &str) -> Result<(fs::File, PathBuf)> {
	let prefix = SmolStr::from(prefix);
	let suffix = SmolStr::from(suffix);
	tokio::task::spawn_blocking(move || {
		let (std_file, path) = tempfile::Builder::new()
			.prefix(&prefix)
			.suffix(&suffix)
			.tempfile()
			.context("Failed to create temp file")?
			.keep()
			.context("Failed to persist temp file")?;
		let file = tokio::fs::File::from_std(std_file);
		anyhow::Ok((file, path))
	})
	.await
	.context("Join Error")?
}
