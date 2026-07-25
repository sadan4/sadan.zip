use std::{
	collections::HashMap,
	fmt::{Display, Write as _},
	future::{self, Ready},
	io::Write as _,
	iter,
	path::{Path, PathBuf},
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Context as _, Result, bail};
use clap::{Parser, ValueEnum};
use discord_scraper::{NoProgress, make_reqwest_client, scrape_full_bundle};
use explorer_server_core::Channel;
use explorer_types::{FullBundle, ModuleId};
use itertools::Itertools;
use macros::{SlashArgs, command, executor};
use memchr::memmem::Finder;
use miette::{Diagnostic, NamedSource, Severity};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator as _};
use reporter::{
	SourceWrapper,
	diag::ReporterError,
	fetcher::ScrapedOutput,
	reporter::Msg,
	util::{MultiProgressWrapper, Stage, debug_module_url},
	vc::Plugin,
};
use reqwest_middleware::ClientWithMiddleware;
use serenity::all::{
	Color,
	Context,
	CreateEmbed,
	CreateEmbedFooter,
	prelude::TypeMapKey,
};
use smol_str::{SmolStr, ToSmolStr};
use tokio::{
	process,
	sync::{Mutex, RwLock},
	task::JoinSet,
	try_join,
};
use tracing::info;

use crate::{
	BotConfig,
	fw::{
		Command,
		CommandCtx,
		CommandExecutor,
		CommandFramework,
		OpaqueExecutor,
	},
};

#[command]
#[group]
#[sub_cmds(find_module_factory, test_pr)]
#[init = register_wp_ctx]
#[early_init]
struct Webpack;

async fn register_wp_ctx(
	fw: &CommandFramework,
) -> Result<impl CommandExecutor + Send + Sync + 'static> {
	init_webpack_context(fw)
		.await
		.context("Failed to initialize webpack context")?;
	Ok(OpaqueExecutor::dummy_executor())
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BranchArg {
	Canary,
	Stable,
	Both,
}

#[derive(Parser, SlashArgs)]
/// Search all of discords webpack modules for a given query stringj
struct FindModuleFactoryArgs {
	#[arg()]
	/// The query string. it will be canonicalized
	query: String,
}

#[derive(Clone)]
struct WebpackContext {
	stable_build: Arc<RwLock<FullBundle>>,
	canary_build: Arc<RwLock<FullBundle>>,
	client: Arc<ClientWithMiddleware>,
}

impl TypeMapKey for WebpackContext {
	type Value = WebpackContext;
}

#[command]
#[arg_parser = FindModuleFactoryArgs]
#[slash_args]
async fn find_module_factory(
	args: FindModuleFactoryArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	_: &Command,
	fw: &CommandFramework,
) -> Result<()> {
	let state = fw
		.get_data::<WebpackContext>()
		.await
		.context("Failed to get webpack context")?;
	let stable_finder = Finder::new(&args.query).into_owned();
	let canray_finder = stable_finder.clone();
	let stable_build = Arc::clone(&state.stable_build);
	let canary_build = Arc::clone(&state.canary_build);
	let stable_fut = tokio::task::spawn_blocking(move || {
		collect_module_matches(
			&stable_build.blocking_read().modules,
			&stable_finder,
		)
	});
	let canary_fut = tokio::task::spawn_blocking(move || {
		collect_module_matches(
			&canary_build.blocking_read().modules,
			&canray_finder,
		)
	});
	let (stable_matches, canary_matches) = try_join!(stable_fut, canary_fut)
		.context("Failed to join module search tasks")?;
	let mut r = String::new();
	write!(
		r,
		"Stable matches (build number: {}): ",
		state
			.stable_build
			.read()
			.await
			.metadata
			.build_number
	)
	.unwrap();
	for (i, id) in stable_matches.iter().enumerate() {
		if i != 0 {
			r.push_str(", ");
		}
		write!(r, "{id}").unwrap();
	}
	writeln!(r).unwrap();
	write!(
		r,
		"Canary matches (build number: {}): ",
		state
			.canary_build
			.read()
			.await
			.metadata
			.build_number
	)
	.unwrap();
	for (i, id) in canary_matches.iter().enumerate() {
		if i != 0 {
			r.push_str(", ");
		}
		write!(r, "{id}").unwrap();
	}

	cctx.reply(ctx, r)
		.await
		.context("Failed to send reply")?;
	Ok(())
}

async fn scrape_branch(
	client: Arc<ClientWithMiddleware>,
	channel: Channel,
) -> Result<FullBundle> {
	let html = reporter::fetcher::fetch_index(&client, channel)
		.await
		.with_context(|| {
			format!("Failed to fetch index.html for {channel:?} branch")
		})?;
	let bundle = scrape_full_bundle(
		html.text.as_ref(),
		channel,
		html.build_hash,
		client.clone(),
		Arc::new(NoProgress),
	)
	.await
	.with_context(|| {
		format!("Failed to scrape full bundle for {channel:?} branch")
	})?;
	Ok(bundle)
}
async fn init_webpack_context(fw: &CommandFramework) -> Result<()> {
	let client =
		make_reqwest_client().context("Failed to make reqwest client")?;
	let client1 = client.clone();
	let stable_fut =
		tokio::spawn(scrape_branch(client.clone(), Channel::Stable));
	let canary_fut = tokio::spawn(scrape_branch(client1, Channel::Canary));
	let stable_build = stable_fut
		.await
		.context("Join error")??;
	let canary_build = canary_fut
		.await
		.context("Join error")??;
	let ctx = WebpackContext {
		stable_build: Arc::new(RwLock::new(stable_build)),
		canary_build: Arc::new(RwLock::new(canary_build)),
		client,
	};
	fw.set_data::<WebpackContext>(ctx).await;
	Ok(())
}
fn collect_module_matches(
	modules: &ScrapedOutput,
	query: &Finder<'_>,
) -> Vec<ModuleId> {
	modules
		.par_iter()
		.filter_map(|(id, module)| {
			if query.find(module.as_bytes()).is_some() {
				Some(*id)
			} else {
				None
			}
		})
		.collect::<Vec<_>>()
}

#[derive(Parser, SlashArgs)]
struct TestPrArgs {
	pr_number: u64,
}

static REPO_LOCK: Mutex<()> = Mutex::const_new(());

#[command]
#[arg_parser = TestPrArgs]
#[slash_args]
async fn test_pr(
	args: TestPrArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	_: &Command,
	fw: &CommandFramework,
) -> Result<()> {
	let dur = Instant::now();
	cctx.defer(ctx)
		.await
		.context("Failed to defer")?;
	let venord_dir = ctx
		.data
		.read()
		.await
		.get::<BotConfig>()
		.unwrap()
		.vencord_path
		.clone();
	let plugins = {
		let _ = REPO_LOCK.lock().await;
		let cmd = process::Command::new("git")
			.arg("fetch")
			.arg("origin")
			.arg(format!("pull/{}/head", args.pr_number))
			.current_dir(&venord_dir)
			.status()
			.await
			.context("Failed to fetch pr branch")?;
		if !cmd.success() {
			bail!("Failed to fetch pr branch: git exited with status {cmd}");
		}
		let cmd = process::Command::new("git")
			.arg("checkout")
			.arg("FETCH_HEAD")
			.current_dir(&venord_dir)
			.status()
			.await
			.context("Failed to checkout pr branch")?;
		if !cmd.success() {
			bail!(
				"Failed to checkout pr {} branch: git exited with status {cmd}",
				args.pr_number
			);
		}

		let vencord_dir = PathBuf::from(venord_dir);
		let opts = tokio::task::spawn_blocking(move || {
			anyhow::Ok(reporter::vc::VencordOpts {
				plugin_dirs: reporter::vc::infer_plugin_dirs(
					vencord_dir.as_ref(),
				)
				.context("Failed to infer plugin dirs")?,
				vencord_dir,
			})
		})
		.await
		.context("Join Error")??;
		reporter::vc::collect_patches(opts, Stage::hidden())
			.await
			.context("Failed to collect patches")?
	};
	let stable_build = fw
		.get_data::<WebpackContext>()
		.await
		.context("Loading discord bundles, please wait")?
		.stable_build;
	let stable_build = stable_build.read().await;
	let stable_build_hash = stable_build.metadata.build_hash.clone();
	let stable_build_number = stable_build.metadata.build_number;
	let stable_build_modules = Arc::new(stable_build.modules.clone());
	drop(stable_build);
	let plugins = Arc::new(plugins);
	let mut rx = reporter::reporter::report_broken_patches(
		Channel::Stable,
		stable_build_modules,
		plugins.clone(),
	);
	let mut errs: Vec<ReporterError> = Vec::new();
	while let Some(msg) = rx.recv().await {
		match msg {
			Msg::RequestProgressBar(sender) => sender
				.send(MultiProgressWrapper::null_bar())
				.unwrap(),
			Msg::Error(e) => 'm: {
				fn is_bad_vencord_patch(
					plugins: &[Plugin],
					e: &ReporterError,
				) -> bool {
					const BAD_CAUSE: &str =
						"/\\.openNativeAppModal\\(.{0,50}?\\.DEEP_LINK/";
					let cause_span = e.cause_span();
					if cause_span.len() as usize == BAD_CAUSE.len() {
						let plugin_str = &plugins[e.plugin_id() as usize]
							.entry_source[cause_span.offset() as usize
							..(cause_span.offset() + cause_span.len()) as usize];
						plugin_str == BAD_CAUSE
					} else {
						false
					}
				}
				if e.is_no_warn()
					|| e.severity() == Some(Severity::Warning)
					|| is_bad_vencord_patch(&plugins, &e)
				{
					break 'm;
				}
				errs.push(e);
			}
			Msg::Done(_) => {
				break;
			}
		}
	}
	let mut embed = CreateEmbed::new()
		.title(format!(
			"Broken patches for pr {} (build number: {})",
			args.pr_number, stable_build_number
		))
		.footer(CreateEmbedFooter::new(format!(
			"Tested in {:?}",
			dur.elapsed()
		)))
		.color(Color::RED);
	if errs.is_empty() {
		embed = embed.color(Color::DARK_GREEN);
		embed = embed.field("No errors found!", "All patches that can be statically tested are working", false);
	} else {
		errs.sort_unstable_by_key(ReporterError::plugin_id);
		let fields_iter = errs.into_iter().map(|err| {
			let plugin = &plugins[err.plugin_id() as usize];
			let plugin_path = plugin
				.entry_point
				.components()
				.tail(2)
				.map(|c| Path::new(c.as_os_str()).display())
				.join("/");
			let reason = err.code().unwrap();
			let title = format!("`{plugin_path}` {reason}");
			let mut cause = String::new();
			if let Some(mid) = err.module_id() {
				let module_link = debug_module_url(mid, &stable_build_hash);
				writeln!(cause, "Module [`{mid}`]({module_link})").unwrap();
			}
			let source_span = err.cause_span();
			let source_cause_snippet = &plugin.entry_source[source_span.offset()
				as usize
				..(source_span.offset() + source_span.len()) as usize];
			write!(cause, "```js\n{source_cause_snippet}\n```").unwrap();
			(title, cause, false)
		});
		embed = embed.fields(fields_iter);
	}
	let elapsed = dur.elapsed();
	info!("Tested PR {} in {:?}", args.pr_number, elapsed);
	cctx.followup_embed(ctx, iter::once(embed))
		.await
		.context("Failed to follup up with embed")?;
	Ok(())
}
