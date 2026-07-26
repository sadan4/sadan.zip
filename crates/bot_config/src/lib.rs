use derive_more::Debug;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serenity::all::{EmojiId, GuildId, ReactionType, UserId};

fn wrap_schema<T: ?Sized + JsonSchema>(g: &mut SchemaGenerator) -> Schema {
	g.subschema_for::<T>()
}

#[derive(Serialize, Default, Clone, Deserialize, Debug, JsonSchema)]
/// Bot config. Read from `.bot.config.json`
pub struct Config {
	#[debug("REDACTED")]
	/// The bot token
	pub token: String,
	#[schemars(schema_with = "wrap_schema::<String>")]
	pub home_guild_id: GuildId,
	/// If true, clap errors will be rendered in an ANSI codeblock
	///
	/// if false, they will just use plain text
	pub use_ansi_clap_errors: bool,
	/// Users who can use owner-only commands
	#[schemars(schema_with = "wrap_schema::<Vec<String>>")]
	pub bot_owners: Vec<UserId>,
	/// The path of the vencord repo, used for testing PRs
	pub vencord_path: String,
	/// The url for the build archive
	pub build_archive_url: String,
	/// If true, cache local builds for faster startup time.
	pub use_local_build_cache: bool,
	/// The emojis available for the bot to access
	pub emojis: Emojis
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, JsonSchema)]
pub struct Emojis {
	/// A 1x1 transparent image
	pub empty: EmojiDef,
}

/// an emoji available for the bot to access
#[derive(Serialize, Default, Deserialize, Clone, Debug, JsonSchema)]
pub struct EmojiDef {
	#[schemars(schema_with = "wrap_schema::<String>")]
	id: EmojiId,
	#[serde(default)]
	#[schemars(schema_with = "wrap_schema::<Option<bool>>")]
	animated: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	name: Option<String>,
}

impl From<EmojiDef> for ReactionType {
	fn from(value: EmojiDef) -> Self {
		Self::Custom {
			id: value.id,
			animated: value.animated,
			name: value.name,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn doesnt_print_token() {
		let cfg: Config = Config {
			token: String::from("MyToken"),
			..Config::default()
		};
		let dbg_repr = format!("{cfg:?}");
		assert!(!dbg_repr.contains("MyToken"));
	}
}
