use std::{
	future::{self, Ready},
	time::Duration,
};

use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{command, executor};
use serenity::all::{CommandOptionType, Context};
use tokio::{
	select,
	sync::{mpsc, oneshot},
	time::sleep,
};
use tracing::{debug, warn};

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

struct QalcPacket {
	expr: String,
	tx: oneshot::Sender<Result<String, cxx::Exception>>,
}

#[command]
#[arg_parser = QalcArgs]
#[init = Qalc::new]
#[slash_args]
struct Qalc {
	tx: mpsc::Sender<QalcPacket>,
}

impl Qalc {
	const CHANNEL_SIZE: usize = 256;

	fn qalc_thread(mut rx: mpsc::Receiver<QalcPacket>) {
		let mut qalc = qalc::ffi::Qalculator::create();
		let mut qalc = qalc.as_mut().unwrap();
		qalc.as_mut()
			.allow_impure_expressions(false);
		qalc.as_mut().enable_sandboxing();
		if !qalc.as_mut().load_exchange_rates() {
			warn!("Failed to load exchange rates");
		}
		if !qalc.as_mut().load_global_defs() {
			warn!("Failed to load global defs");
		}
		if !qalc.as_mut().load_local_defs() {
			warn!("Failed to load local defs");
		}
		while let Some(QalcPacket { expr, tx }) = rx.blocking_recv() {
			let res = qalc.as_mut().calculate_and_print(&expr);
			match tx.send(res) {
				Ok(()) => {}
				Err(_) => {
					warn!(
						"Failed to send qalc result to channel, receiver dropped"
					);
				}
			}
		}
		warn!("qalc thread exiting, channel closed");
	}

	fn new(_: &CommandFramework) -> Ready<Result<Self>> {
		let (tx, rx) = mpsc::channel(Self::CHANNEL_SIZE);
		tokio::task::spawn_blocking(move || Self::qalc_thread(rx));
		future::ready(Ok(Self { tx }))
	}

	async fn run(&self, expr: String) -> Result<String> {
		const TIMEOUT_DUR: Duration = Duration::from_secs(20);
		let (tx, rx) = oneshot::channel();
		self.tx
			.send(QalcPacket { expr, tx })
			.await
			.context("Failed to send qalc packet")?;
		let timeout = sleep(TIMEOUT_DUR);
		let res: Option<
			Result<Result<String, cxx::Exception>, oneshot::error::RecvError>,
		> = select! {
			res = rx => Some(res),
			() = timeout => None,
		};
		let res = res
			.context("qalc timeout after 20s")?
			.context("Failed to recv qalc result")?
			.context("qalc failed to evaluate expression")?;
		Ok(res)
	}
}

#[executor]
async fn qalc(
	args: QalcArgs,
	state: &Qalc,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let expr = args.expr.join(" ");
	debug!("Evaluating expression: {}", expr);
	let mut res = state.run(expr).await?;
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
