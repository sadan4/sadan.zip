use std::{
	borrow::Cow,
	debug_assert_matches,
	fmt::Write as _,
	io::Write,
	mem,
	sync::LazyLock,
};

use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{SlashArgs, command};
use memchr::memmem::Finder;
use serde::Serialize;
use serenity::all::{
	CommandOptionType,
	Context,
	CreateContainerComponent,
	CreateMediaGallery,
	CreateMediaGalleryItem,
	CreateSeparator,
	CreateTextDisplay,
	CreateUnfurledMediaItem,
};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{error, info};

use crate::{
	fw::{CommandCtx, CommandFramework, Paigeinator, SlashOption, SlashSchema},
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

static INPUT_POD_ID: &str = "Input";

static CODEBLOCK_FINDER: LazyLock<Finder<'static>> =
	LazyLock::new(|| Finder::new(b"```"));

fn cleanblocks(txt: &mut str) {
	// SAFETY: This is safe. we only ever replace single ascii bytes with other single ascii bytes
	// which will never create invalid UTF-8
	let bts = unsafe { txt.as_bytes_mut() };
	let f = &*CODEBLOCK_FINDER;
	while let Some(pos) = f.find(bts) {
		debug_assert_eq!(&bts[pos..pos + 3], b"```");
		bts[pos] = b'\'';
		bts[pos + 1] = b'\'';
		bts[pos + 2] = b'\'';
		debug_assert_eq!(&bts[pos..pos + 3], b"'''");
	}
}

fn build_ui(
	d: model::Response,
) -> Vec<Cow<'static, [CreateContainerComponent<'static>]>> {
	#[derive(Debug)]
	struct OutData {
		title: String,
		desc: String,
		image: Option<String>,
		id: String,
	}
	let mut outdata = Vec::new();
	let mut input = None;
	for mut pod in d.query_result.pods {
		let mut od = OutData {
			id: pod.id,
			title: pod.title,
			image: pod
				.subpods
				.first_mut()
				.map(|sp| mem::take(&mut sp.img.src)),
			desc: String::new(),
		};
		for sp in pod.subpods {
			writeln!(od.desc, "{}", sp.plaintext).unwrap();
		}
		while od.desc.ends_with('\n') {
			od.desc.pop();
		}
		cleanblocks(&mut od.desc);
		od.desc.insert_str(0, "```\n");
		od.desc.push_str("\n```");
		if od.id == INPUT_POD_ID {
			debug_assert_matches!(input, None, "Multiple input pods found");
			input = Some(od);
		} else {
			outdata.push(od);
		}
	}
	let mut cpts = Vec::new();
	for mut d in outdata {
		let mut page = Vec::new();
		if let Some(i) = &input {
			page.push(CreateContainerComponent::TextDisplay(
				CreateTextDisplay::new(i.desc.clone()),
			));
			page.push(CreateContainerComponent::Separator(
				CreateSeparator::new(),
			));
		}
		d.title.insert_str(0, "## ");
		page.push(CreateContainerComponent::TextDisplay(
			CreateTextDisplay::new(d.title),
		));
		d.desc.truncate(1000);
		cleanblocks(&mut d.desc);
		d.desc.insert_str(0, "```\n");
		d.desc.push_str("\n```");
		page.push(CreateContainerComponent::TextDisplay(
			CreateTextDisplay::new(d.desc),
		));
		if let Some(i) = d.image {
			page.push(CreateContainerComponent::MediaGallery(
				CreateMediaGallery::new(vec![CreateMediaGalleryItem::new(
					CreateUnfurledMediaItem::new(i),
				)]),
			));
		}
		cpts.push(page.into());
	}
	cpts
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
	let pages = build_ui(r);
	Paigeinator::new()
		.with_pages(pages)
		.with_creator(cctx.author().id)
		.run(ctx, cctx)
		.await
		.context("Paigeinator failed for wa")?;
	Ok(())
}
