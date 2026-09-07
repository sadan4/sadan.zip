#![feature(result_option_map_or_default)]
pub mod cache;
pub mod cmds;
pub mod diag;
pub mod fetcher;
pub mod reporter;
pub mod util;
pub mod vc;

use std::sync::Arc;

use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use derive_more::{From, Into};
use explorer_server_core::Channel;
use miette::{
	GraphicalReportHandler,
	SourceCode,
	SpanContents,
	highlighters::SyntectHighlighter,
};
use terminal_size::{Width, terminal_size};

use crate::{
	fetcher::FetchOpts,
	vc::{Plugin, VencordOpts},
};

#[derive(Parser, Clone)]
#[command(version, about)]
pub struct Cli {
	#[command(flatten)]
	pub vc_opts: VencordOpts,
	#[command(flatten)]
	pub fetch_opts: FetchOpts,
	/// Dump the contents of any module involved in an error, before any transformations, to `$PWD/{Stable, Canary}/<module_id>.js`
	#[arg(long, default_value_t = false)]
	pub dump_on_error: bool,
	/// print debug logs
	#[arg(short, long, action = clap::ArgAction::Count)]
	pub verbose: u8,
	/// Do not print reporter warnings, only print errors.
	///
	/// This is not the same thing as a patch being noWarn
	#[arg(long, default_value_t)]
	pub no_warnings: bool,
	/// Generate shell completions
	#[arg(long, value_enum)]
	pub completions: Option<Shell>,
	#[command(subcommand)]
	pub cmd: cmds::Cmd,
}

impl Cli {
	pub const fn is_debug(&self) -> bool {
		self.verbose >= 1
	}
	pub const fn is_trace(&self) -> bool {
		self.verbose >= 2
	}
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Branch {
	Stable,
	Canary,
}

impl From<Branch> for Channel {
	fn from(value: Branch) -> Self {
		match value {
			Branch::Stable => Self::Stable,
			Branch::Canary => Self::Canary,
		}
	}
}

#[derive(From, Into)]
pub struct SourceWrapper(pub Arc<Vec<Plugin>>, pub u16);

impl SourceCode for SourceWrapper {
	fn read_span<'a>(
		&'a self,
		span: &miette::SourceSpan,
		context_lines_before: usize,
		context_lines_after: usize,
	) -> Result<Box<dyn SpanContents<'a> + 'a>, miette::MietteError> {
		self.0[self.1 as usize]
			.entry_source
			.read_span(span, context_lines_before, context_lines_after)
	}
}

pub fn install_miette_hook() {
	static SYNTAXES: &[u8] = include_bytes!("./syntaxes.bin");
	static THEMES: &[u8] = include_bytes!("./theme.bin");
	miette::set_hook(Box::new(|_| {
		let syntax_raw = zstd::decode_all(SYNTAXES)
			.expect("failed to decompress syntax data");
		let syntaxes = bitcode::deserialize(&syntax_raw)
			.expect("failed to deserialize syntax data");
		let themes_raw =
			zstd::decode_all(THEMES).expect("failed to decompress theme data");
		let themes = bitcode::deserialize(&themes_raw)
			.expect("failed to deserialize theme data");

		Box::new(
			GraphicalReportHandler::new()
				.with_width(
					terminal_size()
						.map_or(80, |(Width(width), _)| usize::from(width)),
				)
				.with_cause_chain()
				.with_syntax_highlighting(SyntectHighlighter::new(
					syntaxes, themes, false,
				)),
		)
	}))
	.expect("Failed to set miette hook");
}
