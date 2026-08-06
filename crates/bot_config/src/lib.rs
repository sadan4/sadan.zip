use std::{fmt::Display, path::PathBuf};

use derive_more::Debug;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serenity::{
	all::{EmojiId, GuildId, ReactionType, UserId},
	small_fixed_array::FixedString,
};
use typesize::derive::TypeSize;

fn wrap_schema<T: ?Sized + JsonSchema>(g: &mut SchemaGenerator) -> Schema {
	g.subschema_for::<T>()
}

#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
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
	/// The API key for `WolframAplha`, used for the wolfram command
	#[debug("REDACTED")]
	pub wolfram_api_key: String,
	/// Path to the standalone qalc sandbox worker binary. Spawned as a fresh
	/// process (instead of forking the bot) so its memory footprint stays small.
	pub qalc_worker_path: PathBuf,
	/// Assets used in the bot, such as images and videos
	pub assets: Assets,
}

#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
/// Assets used in the bot, such as images and videos
pub struct Assets {
	/// The emojis available for the bot to access
	pub emojis: Emojis,
	/// The GIF templates available for the bot to use
	pub gif_templates: GifTemplates,
}

/// the gif templates available for the bot to use
#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
pub struct GifTemplates {
	pub hammer: GifTemplate,
}

/// a GIF template
#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
pub struct GifTemplate {
	/// Path to the data.json file for the GIF template
	/// It should follow the schema in [`GifTemplateData`]
	pub data_path: PathBuf,
	/// Path to dir with frames
	pub frames_path: PathBuf,
}

#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
pub struct GifTemplateData {
	/// The number of frames in the GIF template
	/// should match the number of frames in the [`frames path`](GifTemplate::frames_path)
	pub num_frames: u32,
	/// Delay between frames in milliseconds
	pub delay: u32,
	/// Width of final GIF in pixels
	pub width: u32,
	/// Height of final GIF in pixels
	pub height: u32,
	/// Images to inject into the GIF template, with their position and size
	pub injection: Vec<GifTemplateInjection>,
	/// prefix for the frame files, e.g. `frame_` for `frame_0.png`, `frame_1.png`, etc.
	pub frame_prefix: String,
	/// the type of the frame files, e.g. `png` for `frame_0.png`, `frame_1.png`, etc.
	pub file_type: FrameType,
	/// Quality of the GIF, in the range [1, 30]
	///
	/// 1 is the best quality, 30 is the fastest
	pub gif_quality: u8,
}

/// the type of frames
#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
pub enum FrameType {
	#[default]
	Unknown,
	/// Png frames
	Png,
}

impl FrameType {
	pub fn ext(&self) -> impl Display + 'static {
		match self {
			Self::Unknown => "",
			Self::Png => ".png",
		}
	}
}

#[derive(
	Serialize, Default, Clone, Deserialize, Debug, JsonSchema, TypeSize,
)]
pub struct GifTemplateInjection {
	pub x: u32,
	pub y: u32,
	pub width: u32,
	pub height: u32,
}

#[derive(
	Serialize, Deserialize, Default, Clone, Debug, JsonSchema, TypeSize,
)]
pub struct Emojis {
	/// A 1x1 transparent image
	pub empty: EmojiDef,
}

/// an emoji available for the bot to access
#[derive(
	Serialize, Default, Deserialize, Clone, Debug, JsonSchema, TypeSize,
)]
pub struct EmojiDef {
	#[schemars(schema_with = "wrap_schema::<String>")]
	id: EmojiId,
	#[serde(default)]
	#[schemars(schema_with = "wrap_schema::<Option<bool>>")]
	animated: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schemars(schema_with = "wrap_schema::<Option<String>>")]
	name: Option<FixedString<u8>>,
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

	#[test]
	fn doesnt_print_wolfram_key() {
		let cfg: Config = Config {
			wolfram_api_key: String::from("MyWolframKey"),
			..Config::default()
		};
		let dbg_repr = format!("{cfg:?}");
		assert!(!dbg_repr.contains("MyWolframKey"));
	}
}
