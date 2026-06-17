pub mod cmds;
pub mod diag;
pub mod err;
pub mod fetcher;
pub mod reporter;
pub mod util;
pub mod vc;

use std::sync::Arc;

use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use derive_more::{From, Into};
use explorer_server_core::Channel;
use miette::SourceCode;

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
	/// Do not print reporter warnings, only print errors.
	///
	/// This is not the same thing as a patch being noWarn
	#[arg(long, default_value_t = false)]
	pub no_warnings: bool,
	/// Generate shell completions
	#[arg(long, value_enum)]
	pub completions: Option<Shell>,
	#[command(subcommand)]
	pub cmd: cmds::Cmd,
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
pub struct SourceWrapper(Arc<Vec<Plugin>>, u16);

impl SourceCode for SourceWrapper {
	fn read_span<'a>(
		&'a self,
		span: &miette::SourceSpan,
		context_lines_before: usize,
		context_lines_after: usize,
	) -> std::result::Result<miette::MietteSpanContents<'a>, miette::MietteError>
	{
		self.0[self.1 as usize]
			.entry_source
			.read_span(span, context_lines_before, context_lines_after)
	}

	fn name(&self) -> Option<&str> {
		self.0[self.1 as usize]
			.entry_point
			.to_str()
	}
}
