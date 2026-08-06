use std::{future, time::Duration};

use anyhow::{Context as _, Result, anyhow};
use clap::Parser;
use macros::{command, executor};
use qalc_sbox::Sandbox;
use serenity::all::{CommandOptionType, Context};
use tokio::time::timeout;
use tracing::debug;

use crate::fw::{CommandCtx, CommandFramework, SlashOption, SlashSchema};

#[derive(Parser)]
struct QalcArgs {
	/// The expression to evaluate
	#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
	expr: Vec<String>,
}

impl SlashSchema for QalcArgs {
	fn slash_options() -> Vec<crate::fw::SlashOption> {
		vec![SlashOption {
			choices: vec![],
			kind: CommandOptionType::String,
			name: "expr",
		}]
	}
}

#[command]
#[arg_parser = QalcArgs]
#[init = Qalc::new]
#[early_init]
#[slash_args]
struct Qalc;

impl Qalc {
	const TIMEOUT_DUR: Duration = Duration::from_secs(20);

	/// Spawn the sandboxed qalculate worker and stash it on the framework so
	/// every invocation shares the one child process.
	///
	/// Stays `async` because the `#[init]` factory awaits it; the body itself
	/// does not need to await.
	#[allow(clippy::unused_async)]
	fn new(fw: &CommandFramework) -> impl Future<Output = Result<Self>> {
		future::ready('f: {
			let sbox = match Sandbox::try_new_exec(&fw.config.qalc_worker_path)
				.context("Failed to spawn qalc sandbox worker")
			{
				Ok(s) => s,
				Err(e) => {
					break 'f Err(e);
				}
			};
			fw.init_qalc_worker(sbox);
			anyhow::Ok(Self)
		})
	}
}

async fn run(sbox: &Sandbox, expr: String) -> Result<String> {
	let res = timeout(Qalc::TIMEOUT_DUR, sbox.eval(expr))
		.await
		.context("qalc timeout after 20s")?
		// outer error: the request could not be round-tripped to the sandbox
		.map_err(|e| anyhow!("qalc sandbox unavailable: {e}"))?
		// inner error: libqalculate reported an evaluation failure
		.map_err(|e| anyhow!("qalc failed to evaluate expression: {e}"))?;
	Ok(res)
}

#[executor]
async fn qalc(
	args: QalcArgs,
	_state: &Qalc,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let sbox = fw
		.get_qalc_worker()
		.context("qalc worker is not initialized")?;
	let expr = args.expr.join(" ");
	debug!("Evaluating expression: {}", expr);
	let mut res = run(&sbox, expr).await?;
	res.insert_str(0, "```\n");
	if !res.ends_with('\n') {
		res.push('\n');
	}
	res.push_str("```");
	cctx.followup_text(ctx, res)
		.await
		.context("Failed to send followup reply")?;
	Ok(())
}
