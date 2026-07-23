use std::str::FromStr;

use anyhow::{Result, anyhow, bail};
use derive_more::{Deref, Display, From, Into};
use serenity::all::{Context, Message, UserId};
use tokio::task_local;

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
