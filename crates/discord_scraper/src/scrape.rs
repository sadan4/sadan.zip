use std::{
	collections::HashMap,
	mem,
	sync::{
		Arc,
		LazyLock,
		atomic::{AtomicUsize, Ordering},
	},
	thread,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, bail};
use dashmap::DashMap;
use explorer_server_core::{Channel, asset_url};
use explorer_types::{BundleMetadata, FullBundle, ModuleId};
use http::StatusCode;
use memchr::memmem::Finder;
use oxc_allocator::AllocatorPool;
use reqwest_middleware::ClientWithMiddleware;
use tokio::{
	sync::Semaphore,
	task::{self, JoinSet},
};
use tracing::{debug, info, trace};
use webpack_chunk_parser::{
	JsHashEntry,
	WebpackLazyChunkParser,
	WebpackMainChunkParser,
	base::WebpackChunkParser,
};

use crate::{
	bundle_parser::parse_bundle,
	html_parser::{ParsedHtml, parse_html},
	progress::ScrapeProgress,
	util::ByteStr,
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

pub struct JsScraper {
	inner: Arc<JsScraperInner>,
	chunk_futures: JoinSet<Result<()>>,
	num_extra_chunks: usize,
	global_env_text: String,
	web_js_url: String,
	extra_chunks: Vec<JsHashEntry>,
}

struct JsScraperInner {
	/// TODO: should this be arc?
	client: Arc<ClientWithMiddleware>,
	pending_limit: Semaphore,
	pool: AllocatorPool,
	module_sources: DashMap<String, Vec<ModuleId>>,
	/// TODO: should this be arc?
	progress: Arc<dyn ScrapeProgress>,
	modules: DashMap<ModuleId, String>,
	channel: Channel,
	total_bytes: AtomicUsize,
}

struct MainChunkData {
	build_number: u32,
	entry_point: Option<ModuleId>,
	num_chunks: usize,
}

impl JsScraper {
	fn new(
		html: &str,
		channel: Channel,
		client: Arc<ClientWithMiddleware>,
		progress: Arc<dyn ScrapeProgress>,
	) -> Result<Self> {
		progress.set_stage("Parsing index HTML");
		let ParsedHtml {
			global_env_text,
			web_js_url,
			extra_chunks,
		} = parse_html(html).context("Failed to parse index HTML")?;
		let num_extra_chunks = extra_chunks.len();
		let pending_limit = Semaphore::const_new(MAX_PENDING_REQUESTS);
		let nproc = thread::available_parallelism().map_or(1, usize::from);
		let pool = AllocatorPool::new(nproc);
		let chunk_futures = JoinSet::<Result<_>>::new();
		// let chunk_futures: Vec<task::JoinHandle<Result<_>>>;
		let modules = DashMap::new();
		let module_sources = DashMap::new();
		Ok(Self {
			inner: Arc::new(JsScraperInner {
				client,
				pending_limit,
				pool,
				module_sources,
				progress,
				modules,
				channel,
				total_bytes: AtomicUsize::new(0),
			}),
			chunk_futures,
			num_extra_chunks,
			global_env_text,
			web_js_url,
			extra_chunks,
		})
	}

	fn spawn_extra_chunk_tasks(&mut self) {
		for extra_chunk in mem::take(&mut self.extra_chunks) {
			self.spawn_scrape_task(extra_chunk);
		}
	}

	fn spawn_main_chunk_tasks(&mut self, tasks: Vec<JsHashEntry>) {
		for entry in tasks {
			self.spawn_scrape_task(entry);
		}
	}

	fn spawn_scrape_task(&mut self, entry: JsHashEntry) {
		let hash = entry.hash.clone(); // O(1) clone
		let inner = self.inner.clone();
		self.chunk_futures.spawn(async move {
			let permit = inner.pending_limit.acquire().await.unwrap();
			let chunk_name = format!("{hash}.js");
			let chunk_url = asset_url(inner.channel, &chunk_name);
			let response = inner.client
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
			inner.total_bytes.fetch_add(chunk_bts.len(), Ordering::SeqCst);
			task::spawn_blocking(move || -> Result<()> {
				if WORKER_FINDER.find(&chunk_bts).is_some() {
					trace!("Skipping worker chunk");
					return Ok(());
				}
				let alloc_guard = inner.pool.get();
				let alloc = &*alloc_guard;
				let chunk_str = str::from_utf8(&chunk_bts)
					.context("Response is not valid UTF8")?;
				let chunk_parser = WebpackLazyChunkParser::try_new(
					alloc, chunk_str,
				)
				.with_context(|| {
					format!(
						"failed to create chunk parser for {entry:?} chunk_url={chunk_url} chunk_name={chunk_name} hash={hash}",
					)
				})?;
				let modules = chunk_parser
					.collect_defined_modules()
					.context("Failed to get modules from chunk")?;
				let mut keys = Vec::with_capacity(modules.size_hint().0);
				for (m_id, src) in modules {
					keys.push(m_id);
					inner.modules.insert(m_id, src);
				}
				inner.module_sources.insert(chunk_name, keys);
				inner.progress.chunk_finished();
				Result::Ok(())
			}).await??;
			Ok(())
		});
	}
	async fn fetch_main_js_chunk(&self) -> Result<ByteStr> {
		let web_js_bytes = self
			.inner
			.client
			.get(asset_url(self.inner.channel, &self.web_js_url))
			.send()
			.await?
			.bytes()
			.await?;
		let bstr = ByteStr::try_from(web_js_bytes)
			.context("Response is not valid utf8")?;
		Ok(bstr)
	}
	fn parse_main_js_chunk(
		&mut self,
		main_js_text: &ByteStr,
	) -> Result<MainChunkData> {
		self.inner
			.total_bytes
			.fetch_add(main_js_text.as_ref().len(), Ordering::SeqCst);
		// weird lifetime issue with spawn_main_chunk_tasks later
		let inner2 = self.inner.clone();
		let alloc_guard = inner2.pool.get();
		let alloc = &*alloc_guard;
		self.inner
			.progress
			.set_stage("Parsing main JS chunk");
		let main_parser =
			WebpackMainChunkParser::try_new(alloc, main_js_text.as_ref())
				.context("Failed to parse main js chunk")?;
		let chunks = main_parser
			.get_js_chunk_hashes()
			.context("Failed to get js chunk hashes")?;
		let num_chunks = chunks.len() + self.num_extra_chunks;
		debug!("found {num_chunks} chunks");
		self.inner
			.progress
			.set_chunk_total(chunks.len());
		self.spawn_main_chunk_tasks(chunks);
		let build_number = main_parser
			.get_build_number()
			.unwrap_or_default()
			.parse()
			.unwrap_or_default();
		let entry_point = main_parser.get_entrypoint_id();
		let main_modules = main_parser
			.collect_defined_modules()
			.context("Failed to get modules from main chunk")?;
		for (m_id, src) in main_modules {
			self.inner.modules.insert(m_id, src);
		}
		Ok(MainChunkData {
			build_number,
			entry_point,
			num_chunks,
		})
	}
	pub async fn scrape(
		html: &str,
		channel: Channel,
		client: Arc<ClientWithMiddleware>,
		progress: Arc<dyn ScrapeProgress>,
	) -> Result<ScrapedModules> {
		let mut scraper = Self::new(html, channel, client, progress)
			.context("Failed to create scraper")?;
		scraper.spawn_extra_chunk_tasks();

		scraper
			.inner
			.progress
			.set_stage("Fetching main JS chunk");
		let main_js_text = scraper
			.fetch_main_js_chunk()
			.await
			.context("Failed to fetch main js chunk")?;
		let MainChunkData {
			build_number,
			entry_point,
			num_chunks,
		} = scraper
			.parse_main_js_chunk(&main_js_text)
			.context("Failed to handle main js chunk")?;
		scraper.chunk_futures.join_all().await;

		info!(
			"collected {} chunks. {} modules. parsed {} bytes of js.",
			num_chunks,
			scraper.inner.modules.len(),
			scraper
				.inner
				.total_bytes
				.load(Ordering::SeqCst),
		);
		let inner = Arc::into_inner(scraper.inner)
			.expect("inner has outstanding references");
		let module_sources = inner.module_sources;
		let modules = inner.modules;

		Ok(ScrapedModules {
			modules: HashMap::from_iter(modules),
			module_sources: HashMap::from_iter(module_sources),
			global_env_text: scraper.global_env_text,
			web_js_url: scraper.web_js_url,
			build_number,
			entry_point,
		})
	}
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
	} = JsScraper::scrape(html, channel, client, progress).await?;

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
