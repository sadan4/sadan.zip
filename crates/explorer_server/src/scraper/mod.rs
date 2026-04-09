pub mod html_parser;

use std::{
	collections::HashMap,
	sync::{Arc, LazyLock},
	thread,
};

use anyhow::{Context as _, Result, bail};
use axum::body::Bytes;
use explorer_server_core::{Channel, asset_url};
use explorer_types::FullBundle;
use itertools::Itertools as _;
use memchr::memmem::Finder;
use oxc_allocator::AllocatorPool;
use reqwest::{Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use tokio::{sync::Semaphore, task};
use tracing::{debug, info, trace};
use webpack_chunk_parser::{
	WebpackLazyChunkParser,
	WebpackMainChunkParser,
	base::WebpackChunkParser,
};

use crate::scraper::html_parser::{ParsedHtml, parse_html};

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
		global_env_text,
		web_js_url,
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
	{
		let alloc_guard = pool.get();
		let alloc = &*alloc_guard;
		let main_parser = task::block_in_place(|| {
			WebpackMainChunkParser::try_new(alloc, web_js_txt)
		})?;
		let chunks = main_parser
			.get_js_chunk_hashes()
			.context("Failed to get JS chunk hashes")?;
		num_chunks = chunks.len();
		debug!("Found {} chunks", chunks.len());
		chunk_futures = chunks
			.into_iter()
			.map(|entry| {
				let hash = entry.hash;
				let client = client.clone();
				let pending_limit = pending_limit.clone();
				let pool = pool.clone();
				task::spawn(async move {
					let permit = pending_limit.acquire().await.unwrap();
					let chunk_url = asset_url(channel, &format!("{hash}.js"));
					let chunk_bts = client
						.get(chunk_url)
						.send()
						.await?
						.bytes()
						.await?;
					drop(permit);
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
							)?;
							let modules = chunk_parser
								.get_defined_modules()
								.context("Failed to get modules from chunk")?;

							Result::Ok(modules)
						})
						.await??;
					Result::Ok(chunk_modules)
				})
			})
			.collect_vec();
		modules = task::block_in_place(move || {
			main_parser
				.get_defined_modules()
				.context("Failed to get modules from main chunk")
		})?;
	};

	for fut in chunk_futures {
		modules.extend(fut.await??);
	}

	info!("collected {} chunks. {} modules", num_chunks, modules.len());
	bail!("TODO");
}
