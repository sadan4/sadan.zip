use std::{
	collections::HashMap,
	sync::{Arc, LazyLock, Mutex},
	thread,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, bail};
use explorer_server_core::{Channel, asset_url};
use explorer_types::{BundleMetadata, FullBundle, ModuleId};
use http::StatusCode;
use itertools::Itertools as _;
use memchr::memmem::Finder;
use oxc_allocator::AllocatorPool;
use reqwest_middleware::ClientWithMiddleware;
use tokio::{sync::Semaphore, task};
use tracing::{debug, info, trace};
use webpack_chunk_parser::{
	WebpackLazyChunkParser,
	WebpackMainChunkParser,
	base::WebpackChunkParser,
};

use crate::{
	bundle_parser::parse_bundle,
	html_parser::{ParsedHtml, parse_html},
	progress::ScrapeProgress,
};

const MAX_PENDING_REQUESTS: usize = 1024;

static WORKER_FINDER: LazyLock<Finder<'static>> =
	LazyLock::new(|| Finder::new(br#".ruid=""#));

pub struct ScrapedModules {
	pub modules: HashMap<ModuleId, String>,
	pub module_sources: HashMap<String, Vec<ModuleId>>,
	pub global_env_text: String,
	pub web_js_url: String,
	pub build_number: u32,
	pub entry_point: Option<ModuleId>,
}

#[expect(clippy::too_many_lines)]
pub async fn scrape_modules(
	html: &str,
	channel: Channel,
	client: Arc<ClientWithMiddleware>,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<ScrapedModules> {
	progress.set_stage("Parsing index HTML");
	let ParsedHtml {
		global_env_text,
		web_js_url,
		extra_chunks,
	} = parse_html(html).context("Failed to parse index HTML")?;
	let pending_limit = Arc::new(Semaphore::const_new(MAX_PENDING_REQUESTS));
	progress.set_stage("Fetching main JS chunk");
	let web_js_bytes = client
		.get(asset_url(channel, &web_js_url))
		.send()
		.await?
		.bytes()
		.await?;
	let web_js_txt =
		str::from_utf8(&web_js_bytes).context("Response is not valid UTF8")?;
	let nproc = thread::available_parallelism().map_or(1, usize::from);
	let pool = Arc::new(AllocatorPool::new(nproc));
	let chunk_futures: Vec<task::JoinHandle<Result<_>>>;
	let mut modules;
	let num_chunks;
	let module_sources: Arc<Mutex<HashMap<String, Vec<ModuleId>>>> =
		Arc::new(Mutex::new(HashMap::new()));
	let build_number;
	let entry_point;
	{
		let alloc_guard = pool.get();
		let alloc = &*alloc_guard;
		progress.set_stage("Parsing main JS chunk");
		let main_parser = task::block_in_place(|| {
			WebpackMainChunkParser::try_new(alloc, web_js_txt)
		})?;
		let chunks = main_parser
			.get_js_chunk_hashes()
			.context("Failed to get JS chunk hashes")?;
		num_chunks = chunks.len() + extra_chunks.len();
		debug!("Found {} chunks", num_chunks);
		progress.set_chunk_total(num_chunks);
		chunk_futures = chunks
			.into_iter()
			.chain(extra_chunks)
			.map(|entry| {
				let hash = entry.hash.clone(); // O(1) clone
				let client = client.clone();
				let pending_limit = pending_limit.clone();
				let pool = pool.clone();
				let module_sources = module_sources.clone();
				let progress = progress.clone();
				task::spawn(async move {
					let permit = pending_limit.acquire().await.unwrap();
					let chunk_name = format!("{hash}.js");
					let chunk_url = asset_url(channel, &chunk_name);
					let response = client
						.get(&chunk_url)
						.send()
						.await?;
					if response.status() == StatusCode::NOT_FOUND {
						bail!("Chunk not found: {chunk_url}");
					}
					let chunk_bts = response
						.bytes()
						.await?;
					drop(permit);
					let chunk_name_2 = chunk_name.clone();
					let chunk_modules =
						task::spawn_blocking(move || -> Result<_> {
							if WORKER_FINDER.find(&chunk_bts).is_some() {
								trace!("Skipping worker chunk");
								return Ok(HashMap::new());
							}
							let alloc_guard = pool.get();
							let alloc = &*alloc_guard;
							let chunk_str = str::from_utf8(&chunk_bts)
								.context("Response is not valid UTF8")?;
							let chunk_parser = WebpackLazyChunkParser::try_new(
								alloc, chunk_str,
							)
							.with_context(|| {
								format!(
									"failed to create chunk parser for {entry:?} chunk_url={chunk_url} chunk_name={chunk_name_2} hash={hash}",
								)
							})?;
							let modules = chunk_parser
								.get_defined_modules()
								.context("Failed to get modules from chunk")?;

							Result::Ok(modules)
						})
						.await??;
					let keys = chunk_modules
						.keys()
						.copied()
						.collect_vec();
					module_sources
						.lock()
						.unwrap()
						.insert(chunk_name, keys);
					progress.chunk_finished();

					Result::Ok(chunk_modules)
				})
			})
			.collect_vec();
		build_number = main_parser
			.get_build_number()
			.unwrap_or_default()
			.parse()
			.unwrap_or_default();
		entry_point = main_parser.get_entrypoint_id();
		modules = main_parser
			.get_defined_modules()
			.context("Failed to get modules from main chunk")?;
	};

	for fut in chunk_futures {
		modules.extend(fut.await??);
	}

	info!("collected {} chunks. {} modules", num_chunks, modules.len());
	drop(pool);

	let module_sources = Arc::into_inner(module_sources)
		.expect("module_sources has outstanding references")
		.into_inner()
		.unwrap();

	Ok(ScrapedModules {
		modules,
		module_sources,
		global_env_text,
		web_js_url,
		build_number,
		entry_point,
	})
}

pub async fn scrape_full_bundle(
	html: &str,
	channel: Channel,
	build_hash: String,
	client: Arc<ClientWithMiddleware>,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<FullBundle> {
	let ScrapedModules {
		mut modules,
		module_sources,
		global_env_text,
		web_js_url: _,
		build_number,
		entry_point,
	} = scrape_modules(html, channel, client, progress).await?;

	// parse_bundle requires modules to be prefixed with "0," so the AST parser
	// sees them as the second element of a sequence expression.
	for code in modules.values_mut() {
		code.insert_str(0, "0,");
	}
	let dep_info = parse_bundle(&modules)?;
	for code in modules.values_mut() {
		code.drain(0..2);
	}

	let current_time = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("Bad System Clock")?
		.as_millis();
	debug_assert!(u64::try_from(current_time).is_ok());
	let first_seen = current_time as u64;

	Ok(FullBundle {
		metadata: BundleMetadata {
			build_hash,
			build_number,
			first_seen,
			entry_point,
			env_var_text: global_env_text,
		},
		dep_info,
		module_sources,
		modules,
	})
}
