use anyhow::{Context as _, Result, bail};
use clap::Parser;
use macros::{SlashArgs, command};
use serde::Serialize;
use serenity::all::{CommandOptionType, Context};
use tracing::info;

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

#[derive(Serialize)]
enum WAUnit {
	Metric,
	// Imperial,
}

#[derive(Serialize)]
enum WAOutput {
	Json,
}

mod model {
	use serde::Deserialize;
	use super::*;

	#[derive(Deserialize, Debug)]
	#[serde(deny_unknown_fields)]
	pub struct Response {
		#[serde(rename = "queryresult")]
		pub query_result: QueryResult,
	}

	#[derive(Deserialize, Debug)]
	#[serde(deny_unknown_fields)]
	pub struct QueryResult {
		pub pods: Vec<Pod>,
		pub success: bool,
		pub error: bool,
		pub numpods: usize,
		pub datatypes: String,
		#[serde(rename = "parsetiming")]
		pub parse_timing: f64,
		#[serde(rename = "parsetimedout")]
		pub parse_timedout: bool,
		id: String,
		#[serde(rename = "kernelId")]
		kernel_id: String,
		#[serde(rename = "processId")]
		process_id: u32,
		version: String,
		#[serde(rename = "inputstring")]
		input_string: String,
		#[serde(rename = "sbsallowed")]
		sbs_allowed: bool,
		#[serde(rename = "parentId")]
		parent_id: String,
		#[serde(rename = "requestId")]
		request_id: String,
	}

	#[derive(Deserialize, Debug)]
	#[serde(deny_unknown_fields)]
	pub struct Image {
		pub alt: String,
		#[serde(rename = "colorinvertable")]
		pub color_invertable: bool,
		#[serde(rename = "contenttype")]
		pub content_type: String,
		pub height: u64,
		pub src: String,
		pub themes: String,
		pub title: String,
		#[serde(rename = "type")]
		pub type_: String,
		pub width: u64,
	}

	#[derive(Deserialize, Debug)]
	#[serde(deny_unknown_fields)]
	pub struct ExpressionTypes {
		pub name: String,
	}

	#[derive(Deserialize, Debug)]
	#[serde(deny_unknown_fields)]
	pub struct Subpod {
		pub img: Image,
		pub plaintext: String,
		pub title: String,
	}

	#[derive(Deserialize, Debug)]
	#[serde(deny_unknown_fields)]
	pub struct Pod {
		pub error: bool,
		#[serde(rename = "expressiontypes")]
		pub expression_types: ExpressionTypes,
		pub id: String,
		#[serde(rename = "numsubpods")]
		pub num_subpods: usize,
		pub position: i64,
		pub scanner: String,
		pub subpods: Vec<Subpod>,
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		fn de(j: &str) -> Response {
			serde_json::from_str(j).unwrap()
		}
		#[test]
		fn test_de() {
			let j = include_str!("./wolfram.json");
			_ = de(j);
		}
	}
}

async fn query_api(fw: &CommandFramework, query: String) -> Result<()> {
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
	tokio::fs::write("wolfram.json", &r).await.unwrap();
	let r: model::Response = serde_json::from_slice(&r)
		.context("Failed to parse response from Wolfram Alpha API")?;
	info!("Wolfram Alpha API response: {r:#?}");
	bail!("TODO");
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
	let query = args.expr.join(" ");
	query_api(fw, query)
		.await
		.context("Failed to query Wolfram Alpha API")?;
	todo!()
}
