mod fix;
mod lint;
mod run;
mod watch;

use clap::Subcommand;
use miette::miette;

use crate::util::MultiProgressWrapper;

#[derive(Subcommand, Default, Clone)]
pub enum Cmd {
	/// Run the reporter once and exit.
	///
	/// Prints results to stderr and exits with 0 if no errors were found.
	///
	/// If only one [branch](crate::fetcher::FetchOpts::branches) is provided, the results will be streamed to stderr
	#[default]
	Run,
	Watch,
	Lint,
	Fix {
		patch_hash: String,
	},
}

pub async fn run(
	cli: super::Cli,
	global_bar: &MultiProgressWrapper,
) -> miette::Result<i8> {
	match cli.cmd {
		Cmd::Run => run::run_reporter(&cli, global_bar)
			.await
			.map(|r| r.num_errs.try_into().unwrap_or(-1))
			.map_err(miette::Report::msg),
		Cmd::Watch => {
			let _ = watch::run_watcher(cli, global_bar).await?;
			unreachable!()
		}
		Cmd::Lint => {
			lint::lint(cli, global_bar).map_err(miette::Report::msg)?;
			Ok(0)
		}
		Cmd::Fix { ref patch_hash } => {
			let hash = u64::from_str_radix(patch_hash, 16).map_err(|_| {
				miette!(
					"Invalid patch hash: `{patch_hash}`. Expected a hex string"
				)
			})?;
			fix::fix(cli, global_bar, hash).await
		}
	}
}
