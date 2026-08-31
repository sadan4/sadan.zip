use std::{
	fmt::Write as _,
	mem,
	path::{Path, PathBuf},
	sync::Arc,
	time::Instant,
};

use anyhow::{Context as _, Result};
use clap::{Parser, ValueEnum};
use derive_more::Display;
use discord_scraper::{NoProgress, scrape_full_bundle};
use explorer_server_core::Channel;
use explorer_types::{FullBundle, ModuleId};
use git2::{
	Cred,
	FetchOptions,
	RemoteCallbacks,
	Repository,
	build::CheckoutBuilder,
};
use itertools::Itertools;
use macros::{SlashArgs, SlashChoices, command};
use memchr::memmem::Finder;
use miette::{Diagnostic, Severity};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator as _};
use reporter::{
	diag::ReporterError,
	fetcher::ScrapedOutput,
	reporter::Msg,
	util::{MultiProgressWrapper, Stage, debug_module_url},
	vc::Plugin,
};
use reqwest_middleware::ClientWithMiddleware;
use serenity::all::{Color, Context, CreateEmbed, CreateEmbedFooter};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, instrument, warn};
use typesize::{TypeSize, derive::TypeSize};

use crate::{
	fw::{CommandCtx, CommandExecutor, CommandFramework, OpaqueExecutor},
	util::trim_heap,
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

#[derive(
	ValueEnum, Display, SlashChoices, Clone, Copy, Debug, PartialEq, Eq, Hash,
)]
enum BranchArg {
	Canary,
	Stable,
	Both,
}

impl BranchArg {
	const fn wants_stable(self) -> bool {
		matches!(self, Self::Stable | Self::Both)
	}

	const fn wants_canary(self) -> bool {
		matches!(self, Self::Canary | Self::Both)
	}
}

#[derive(Parser, SlashArgs)]
/// Search all of discords webpack modules for a given query stringj
struct FindModuleFactoryArgs {
	#[arg()]
	/// The query string. it will be canonicalized
	query: String,
	/// Which branch(es) to search
	#[arg(long, default_value = "both")]
	branch: BranchArg,
}

fn size_arc_rwlock<T: TypeSize>(e: &Arc<RwLock<T>>) -> usize {
	const ARC_OVERHEAD: usize = mem::size_of::<Arc<()>>();
	const RWLOCK_OVERHEAD: usize = mem::size_of::<RwLock<()>>();
	ARC_OVERHEAD + RWLOCK_OVERHEAD + e.blocking_read().get_size()
}

#[derive(Clone, Debug, TypeSize)]
pub struct WebpackContext {
	#[typesize(with = size_arc_rwlock)]
	stable_build: Arc<RwLock<FullBundle>>,
	#[typesize(with = size_arc_rwlock)]
	canary_build: Arc<RwLock<FullBundle>>,
}

#[command]
#[arg_parser = FindModuleFactoryArgs]
#[slash_args]
async fn find_module_factory(
	args: FindModuleFactoryArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	let state = fw
		.get_wp_ctx()
		.context("Failed to get webpack context")?;
	let stable_finder = Finder::new(&args.query).into_owned();
	let canray_finder = stable_finder.clone();
	let stable_build = Arc::clone(&state.stable_build);
	let canary_build = Arc::clone(&state.canary_build);
	let stable_fut = args.branch.wants_stable().then(|| {
		tokio::task::spawn_blocking(move || {
			collect_module_matches(
				&stable_build.blocking_read().modules,
				&stable_finder,
			)
		})
	});
	let canary_fut = args.branch.wants_canary().then(|| {
		tokio::task::spawn_blocking(move || {
			collect_module_matches(
				&canary_build.blocking_read().modules,
				&canray_finder,
			)
		})
	});
	let mut r = String::new();
	if let Some(stable_fut) = stable_fut {
		let stable_matches = stable_fut
			.await
			.context("Failed to join stable module search task")?;
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
	}
	if let Some(canary_fut) = canary_fut {
		let canary_matches = canary_fut
			.await
			.context("Failed to join canary module search task")?;
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
	}

	cctx.reply(ctx, r)
		.await
		.context("Failed to send reply")?;
	Ok(())
}

async fn scrape_branch(
	client: Arc<ClientWithMiddleware>,
	channel: Channel,
	use_cache: bool,
) -> Result<FullBundle> {
	let html = reporter::fetcher::fetch_index(&client, channel)
		.await
		.with_context(|| {
			format!("Failed to fetch index.html for {channel:?} branch")
		})?;
	let cache_key = if use_cache {
		format!("bot-{}-full", html.build_hash)
	} else {
		String::new()
	};
	if use_cache
		&& let Ok(Some(bundle)) = reporter::cache::read(&cache_key).await
	{
		info!("Using cached bundle for {channel:?} branch");
		return Ok(bundle);
	}
	let mut bundle = scrape_full_bundle(
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
	if use_cache
		&& let Err(err) = reporter::cache::write(&cache_key, &bundle, 10).await
	{
		warn!("Failed to write bundle to cache: {err:?}");
	}
	// FullBundle::shrink_to_fit saves ~75MB of ram per bundle
	bundle.shrink_to_fit();
	Ok(bundle)
}
async fn init_webpack_context(fw: &CommandFramework) -> Result<()> {
	let client = fw.http.clone();
	let use_cache = fw.config.use_local_build_cache;
	let stable_fut =
		tokio::spawn(scrape_branch(client.clone(), Channel::Stable, use_cache));
	let canary_fut =
		tokio::spawn(scrape_branch(client, Channel::Canary, use_cache));
	let stable_build = stable_fut
		.await
		.context("Join error")??;
	let canary_build = canary_fut
		.await
		.context("Join error")??;
	let ctx = WebpackContext {
		stable_build: Arc::new(RwLock::new(stable_build)),
		canary_build: Arc::new(RwLock::new(canary_build)),
	};
	fw.init_wp_ctx(ctx);
	// This does not include the hacks for ram savings in scrape_branch
	// we do ~5GiB of allocation per bundle here, this releases ~600MiB of ram to the system per bundle
	trim_heap();
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
	#[arg(long, short, default_value = "both")]
	branch: BranchArg,
	/// Pass a number to test the corresponding PR.
	///
	/// Pass a string to test the corresponding branch.
	target: String,
}

static REPO_LOCK: Mutex<()> = Mutex::const_new(());
const FETCH_HEAD: &str = "FETCH_HEAD";

#[derive(Debug, Display, Clone, Copy)]
enum PrTestTarget<'a> {
	#[display("branch `{_0}`")]
	Branch(&'a str),
	#[display("pr {_0}")]
	Pr(u64),
}

impl<'a> PrTestTarget<'a> {
	/// A bare number selects a PR; anything else is treated as a branch name.
	fn parse(target: &'a str) -> Self {
		target
			.parse::<u64>()
			.map_or(Self::Branch(target), Self::Pr)
	}

	/// The git refspec to fetch for this target.
	fn refspec(self) -> String {
		match self {
			Self::Pr(pr_num) => format!("refs/pull/{pr_num}/head"),
			Self::Branch(branch) => format!("refs/heads/{branch}"),
		}
	}
}

/// blocking by proxy of libgit2
#[instrument]
fn checkout_pr(repo_dir: &Path, target: PrTestTarget<'_>) -> Result<()> {
	let repo = Repository::open(repo_dir).with_context(|| {
		format!("Failed to open repo at {}", repo_dir.display())
	})?;
	debug!("opened repo");
	let mut remote = repo
		.find_remote("origin")
		.context("Failed to find remote `origin`")?;
	debug!("found remote");
	let mut cbs = RemoteCallbacks::new();
	cbs.credentials(|url, username, credential_type| {
		debug!("running credentials callback for url: {url}, username: {username:?}, credential_type: {credential_type:?}");
		Cred::credential_helper(
			&git2::Config::open_default()?,
			url,
			username,
		)
	});
	let mut fo = FetchOptions::new();
	fo.remote_callbacks(cbs);
	debug!("Fetching remote");
	remote
		.fetch(&[target.refspec()], Some(&mut fo), None)
		.with_context(|| format!("Failed to fetch {target}"))?;
	debug!("Fetched remote");
	let commit = repo
		.find_reference(FETCH_HEAD)
		.context("Failed to find FETCH_HEAD after fetch")?
		.peel_to_commit()
		.context("FETCH_HEAD does not point to a commit")?;
	debug!("Found commit {}", commit.id());
	repo.checkout_tree(
		commit.as_object(),
		Some(CheckoutBuilder::new().force()),
	)
	.context("Failed to checkout pr to tree")?;
	debug!("Checked out commit {}", commit.id());
	repo.set_head_detached(commit.id())
		.context("Failed to detach HEAD onto pr commit")?;
	debug!("Detached HEAD onto commit {}", commit.id());
	Ok(())
}

/// Run the static patch report for `plugins` against a single branch's bundle
/// and render the result into an embed.
async fn report_pr_branch(
	channel: Channel,
	build: &Arc<RwLock<FullBundle>>,
	plugins: Arc<Vec<Plugin>>,
	target: PrTestTarget<'_>,
	dur: Instant,
) -> Result<CreateEmbed<'static>> {
	let (build_hash, build_number, modules) = {
		let build = build.read().await;
		(
			build.metadata.build_hash.clone(),
			build.metadata.build_number,
			Arc::new(build.modules.clone()),
		)
	};
	let mut rx = reporter::reporter::report_broken_patches(
		channel,
		modules,
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
							.entry_source
							.as_str()[cause_span];
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
			"Broken {channel:?} patches for {target} (build number: \
			 {build_number})",
		))
		.footer(CreateEmbedFooter::new(format!(
			"Tested in {:?}",
			dur.elapsed()
		)))
		.color(Color::RED);
	if errs.is_empty() {
		embed = embed.color(Color::DARK_GREEN);
		embed = embed.field(
			"No errors found!",
			"All patches that can be statically tested are working",
			false,
		);
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
				let module_link = debug_module_url(mid, &build_hash);
				writeln!(cause, "Module [`{mid}`]({module_link})").unwrap();
			}
			let source_span = err.cause_span();
			let source_cause_snippet =
				&plugin.entry_source.as_str()[source_span];
			write!(cause, "```js\n{source_cause_snippet}\n```").unwrap();
			(title, cause, false)
		});
		embed = embed.fields(fields_iter);
	}
	Ok(embed)
}

#[command]
#[arg_parser = TestPrArgs]
#[slash_args]
async fn test_pr(
	args: TestPrArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	let dur = Instant::now();
	cctx.defer(ctx)
		.await
		.context("Failed to defer")?;
	let venord_dir = fw.config.vencord_path.clone();
	let target = PrTestTarget::parse(&args.target);
	let plugins = {
		_ = REPO_LOCK.lock().await;
		let checkout_target = args.target.clone();
		let opts = tokio::task::spawn_blocking(move || {
			let repo_dir = PathBuf::from(venord_dir);
			checkout_pr(&repo_dir, PrTestTarget::parse(&checkout_target))
				.context("Failed to checkout target")?;
			anyhow::Ok(reporter::vc::VencordOpts {
				plugin_dirs: reporter::vc::infer_plugin_dirs(repo_dir.as_ref())
					.context("Failed to infer plugin dirs")?,
				vencord_dir: repo_dir,
			})
		})
		.await
		.context("Join Error")??;
		reporter::vc::collect_patches(opts, Stage::hidden())
			.await
			.context("Failed to collect patches")?
	};
	let state = fw
		.get_wp_ctx()
		.context("Loading discord bundles, please wait")?;
	let plugins = Arc::new(plugins);
	let mut embeds = Vec::new();
	if args.branch.wants_stable() {
		embeds.push(
			report_pr_branch(
				Channel::Stable,
				&state.stable_build,
				plugins.clone(),
				target,
				dur,
			)
			.await?,
		);
	}
	if args.branch.wants_canary() {
		embeds.push(
			report_pr_branch(
				Channel::Canary,
				&state.canary_build,
				plugins.clone(),
				target,
				dur,
			)
			.await?,
		);
	}
	let elapsed = dur.elapsed();
	info!("Tested {target} in {:?}", elapsed);
	cctx.followup_embed(ctx, embeds)
		.await
		.context("Failed to follup up with embed")?;
	Ok(())
}
