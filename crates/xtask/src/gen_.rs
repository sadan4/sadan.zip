use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Runnable;

mod bot_config;
mod client;
mod client_grammars;
mod discord_intl;
mod indent_cache;
mod monaco_editor;
mod monaco_themes;
mod nix_cargo_hashes;
mod syntax;
mod ts_api;
mod types;
mod update_intl_mappings;

#[derive(Args)]
pub struct Command {
	#[command(subcommand)]
	/// The thing to generate
	target: Target,
}

impl Runnable for Command {
	fn run(&self) -> Result<()> {
		match &self.target {
			Target::IndentCache(c) => c.run(),
			Target::Syntax(c) => c.run(),
			Target::Types(c) => c.run(),
			Target::ClientGrammars(c) => c.run(),
			Target::ClientMonacoThemes(c) => c.run(),
			Target::ClientMonacoEntry(c) => c.run(),
			Target::ClientTsApi(c) => c.run(),
			Target::Client(c) => c.run(),
			Target::DiscordIntl(c) => c.run(),
			Target::BotConfig(c) => c.run(),
			Target::NixCargoHashes(c) => c.run(),
			Target::UpdateIntlMappings(c) => c.run(),
		}
	}
}

#[derive(Subcommand, Clone, Debug)]
enum Target {
	/// Generate the indent cache for `crates/pretty_printer/src/formatted_content_builder.rs`
	IndentCache(indent_cache::Command),
	/// Generate syntax highlighting theme and language definitions for reporter
	Syntax(syntax::Command),
	/// Generate types
	Types(types::Command),
	/// Generate client grammars for syntax highlighting in the browser
	ClientGrammars(client_grammars::Command),
	/// generate and convert vscode themes to monaco themes for monaco-editor
	ClientMonacoThemes(monaco_themes::Command),
	/// Generate the entry point for monaco-editor in the client
	ClientMonacoEntry(monaco_editor::Command),
	/// Generate the keys of node types that are publicly visible in the ts API
	ClientTsApi(ts_api::Command),
	/// Generate all code needed for the client
	Client(client::Command),
	/// Convert the discord intl key mappings to a compressed binary
	/// format for `WebpackAstParser` and other rust crates
	DiscordIntl(discord_intl::Command),
	/// Generate the JSON schema for the bot config (`bot_config::Config`)
	BotConfig(bot_config::Command),
	/// Generate `nix/cargo-output-hashes.nix` from the git dependencies in
	/// `Cargo.lock`
	NixCargoHashes(nix_cargo_hashes::Command),
	/// Update discord intl mappings from url
	UpdateIntlMappings(update_intl_mappings::Command),
}
