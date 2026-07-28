use std::{borrow::Cow, io::Write};

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use macros::{SlashArgs, command};
use serde::Serialize;
use serenity::all::{CommandOptionType, Context};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{error, info};

use crate::{
	fw::{CommandCtx, CommandFramework, SlashOption, SlashSchema},
	util::mktemp,
};

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

#[derive(Serialize)]
enum WAUnit {
	Metric,
	// Imperial,
}

#[derive(Serialize)]
enum WAOutput {
	Json,
}

mod model;

async fn query_api(
	fw: &CommandFramework,
	query: String,
) -> Result<model::Response> {
	#[derive(Serialize)]
	struct WAQueryOpts<'a> {
		appid: &'a str,
		input: &'a str,
		units: WAUnit,
		output: WAOutput,
		ip: &'a str,
	}
	let r = fw
		.http
		.get("https://api.wolframalpha.com/v2/query")
		.query(&WAQueryOpts {
			appid: &fw.config.wolfram_api_key,
			input: &query,
			units: WAUnit::Metric,
			output: WAOutput::Json,
			ip: "169.254.0.1",
		})
		.send()
		.await
		.context("Failed to send request to Wolfram Alpha API")?
		.error_for_status()
		.context("Wolfram Alpha API returned an error")?
		.bytes()
		.await
		.context("Failed to read response from Wolfram Alpha API")?;
	tokio::fs::write("wolfram.json", &r)
		.await
		.unwrap();
	match serde_json::from_slice(&r) {
		Ok(r) => Ok(r),
		Err(e) => {
			const MSG: &str = "Failed to parse response from Wolfram Alpha API";
			let (mut tmp_file, path) = mktemp("wolfram_response_", ".json")
				.await
				.context(
					"Failed to make temp file for Wolfram Alpha API response",
				)?;
			tmp_file
				.write_all(&r)
				.await
				.context("Failed to write api response")?;
			Err(anyhow::Error::from(e).context(format!(
				"{MSG}. Wrote response to {}",
				path.display()
			)))
		}
	}
}

/// Query Wolfram Alpha for an expression and return the result.
#[command]
#[arg_parser = QalcArgs]
#[slash_args]
async fn wolfram(
	args: QalcArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer command")?;
	let query = args.expr.join(" ");
	let r = match query_api(fw, query)
		.await
		.context("Failed to query Wolfram Alpha API")
	{
		Ok(r) => r,
		Err(e) => {
			let mut msg = format!("```\n{e:?}");
			while msg.ends_with('\n') {
				msg.pop();
			}
			msg.push_str("\n```");
			error!("Failed to query Wolfram Alpha API: {e:?}");
			cctx.followup_text(ctx, Cow::Owned(msg))
				.await
				.context("Failed to send error message")?;
			return Ok(());
		}
	};
	todo!()
}
