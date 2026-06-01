mod bundle_parser;
pub mod html_parser;

use std::{
	collections::HashMap,
	fs,
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
use regress::Regex;
use reqwest::Response;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use tokio::{sync::Semaphore, task};
use tracing::{debug, info, trace};
use webpack_chunk_parser::{
	WebpackLazyChunkParser,
	WebpackMainChunkParser,
	base::WebpackChunkParser,
};

use crate::scraper::{
	bundle_parser::parse_bundle,
	html_parser::{ParsedHtml, parse_html},
};

const MAX_PENDING_REQUESTS: usize = 1024;

static WORKER_FINDER: LazyLock<Finder<'static>> =
	LazyLock::new(|| Finder::new(br#".ruid=""#));

static USER_AGENT: &str =
	concat![env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")];

fn make_reqwest_client() -> Result<Arc<ClientWithMiddleware>> {
	let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
	let retry_middleware =
		RetryTransientMiddleware::new_with_policy(retry_policy);
	let client = reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.build()?;
	let client = ClientBuilder::new(client)
		.with(retry_middleware)
		.build();
	Ok(Arc::new(client))
}

pub async fn scrape_build(
	res: Response,
	channel: Channel,
	build_hash: String,
) -> Result<FullBundle> {
	// Parse the index html file to get the urls we need
	let html = res.text().await?;
	let ParsedHtml {
		global_env_text: env_var_text,
		web_js_url,
		extra_chunks,
	} = parse_html(&html).context("Failed to parse index HTML")?;
	let pending_limit = Arc::new(Semaphore::const_new(MAX_PENDING_REQUESTS));
	let client = make_reqwest_client()?;
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
		let main_parser = task::block_in_place(|| {
			WebpackMainChunkParser::try_new(alloc, web_js_txt)
		})?;
		let chunks = main_parser
			.get_js_chunk_hashes()
			.context("Failed to get JS chunk hashes")?;
		num_chunks = chunks.len() + extra_chunks.len();
		debug!("Found {} chunks", num_chunks);
		chunk_futures = chunks
			.into_iter()
			.chain(extra_chunks)
			.map(|entry| {
				let hash = entry.hash.clone(); // O(1) clone
				let client = client.clone();
				let pending_limit = pending_limit.clone();
				let pool = pool.clone();
				let module_sources = module_sources.clone();
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
							let mut modules = chunk_parser
								.get_defined_modules()
								.context("Failed to get modules from chunk")?;
							for code in modules.values_mut() {
								code.insert_str(0, "0,");
							}

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
		let mut tmp_modules = main_parser
			.get_defined_modules()
			.context("Failed to get modules from main chunk")?;
		for code in tmp_modules.values_mut() {
			code.insert_str(0, "0,");
		}
		modules = tmp_modules;
	};

	for fut in chunk_futures {
		modules.extend(fut.await??);
	}

	info!("collected {} chunks. {} modules", num_chunks, modules.len());
	drop(pool);
	let dep_info = parse_bundle(&modules)?;
	// We prepended `0,` to each module so that we can parse them, but we have to remove them now before we store them
	for code in modules.values_mut() {
		code.drain(0..2);
	}

	let module_sources = Arc::into_inner(module_sources)
		.expect("how")
		.into_inner()
		.unwrap();

	let current_time = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("Bad System Clock")?
		.as_millis();
	debug_assert!(u64::try_from(current_time).is_ok());
	let first_seen = current_time as u64;

	let ret = FullBundle {
		metadata: BundleMetadata {
			build_hash,
			build_number,
			first_seen,
			entry_point,
			env_var_text,
		},
		dep_info,
		module_sources,
		modules,
	};

	Ok(ret)
}
