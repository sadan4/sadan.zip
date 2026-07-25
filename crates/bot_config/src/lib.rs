use serde::{Serialize, Deserialize};
use derive_more::Debug;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serenity::all::{GuildId, UserId};

fn wrap_schema<T: ?Sized + JsonSchema>(g: &mut SchemaGenerator) -> Schema {
	g.subschema_for::<T>()
}

#[derive(Serialize, Default, Deserialize, Debug, JsonSchema)]
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
	pub vencord_path:String,
	/// The url for the build archive
	pub build_archive_url: String,
	/// If true, cache local builds for faster startup time.
	pub use_local_build_cache: bool,
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
