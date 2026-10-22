use std::{
	collections::HashMap,
	mem,
	string::FromUtf8Error,
	sync::{
		Arc,
		LazyLock,
		atomic::{AtomicUsize, Ordering},
	},
	thread,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail};
use dashmap::DashMap;
use explorer_server_core::asset_url;
use explorer_types::{
	BundleMetadata,
	Channel,
	FullBundle,
	ModuleId,
	intl::IntlMessages,
};
use http::StatusCode;
use memchr::memmem::Finder;
use oxc_allocator::AllocatorPool;
use reqwest_middleware::ClientWithMiddleware;
use smol_str::{SmolStr, format_smolstr};
use tokio::{
	sync::Semaphore,
	task::{self, JoinSet},
};
use tracing::{debug, info, trace, warn};
use webpack_chunk_parser::{
	ChunkHashEntry,
	WebpackLazyChunkParser,
	WebpackMainChunkParser,
	base::WebpackChunkParser,
};

use crate::{
	experiments::ParsedBundle,
	html_parser::{ParsedHtml, parse_html},
	progress::ScrapeProgress,
	util::ByteStr,
};

const MAX_PENDING_REQUESTS: usize = 1024;

/// rspack <2.0.0 by default adds .ruid to every runtime chunk
/// the only runtime we want is web.js, so we use this to filter
/// out other chunks with their own runtimes (workers, sentry, ...)
static RUNTIME_FINDER: LazyLock<Finder<'static>> =
	LazyLock::new(|| Finder::new(br#".ruid=""#));

pub struct ScrapedModules {
	pub modules: HashMap<ModuleId, String>,
	pub module_sources: HashMap<String, Vec<ModuleId>>,
	pub global_env_text: String,
	pub web_js_url: String,
	pub build_number: u32,
	pub entry_point: Option<ModuleId>,
	pub css_chunks: HashMap<SmolStr, String>,
}

pub struct JsScraper {
	inner: Arc<JsScraperInner>,
	chunk_futures: JoinSet<Result<()>>,
	num_extra_chunks: usize,
	num_extra_css_chunks: usize,
	global_env_text: String,
	web_js_url: String,
	extra_chunks: Vec<ChunkHashEntry>,
extra_css_chunks: Vec<ChunkHashEntry>,
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
	css_chunks: DashMap<SmolStr, String>,
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
			extra_css_chunks,
		} = parse_html(html).context("Failed to parse index HTML")?;
		let num_extra_chunks = extra_chunks.len();
		let num_extra_css_chunks = extra_css_chunks.len();
		let pending_limit = Semaphore::const_new(MAX_PENDING_REQUESTS);
		let nproc = thread::available_parallelism().map_or(1, usize::from);
		let pool = AllocatorPool::new(nproc);
		let chunk_futures = JoinSet::<Result<_>>::new();
		// let chunk_futures: Vec<task::JoinHandle<Result<_>>>;
		let modules = DashMap::new();
		let module_sources = DashMap::new();
		let css_chunks = DashMap::new();
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
				css_chunks
			}),
			chunk_futures,
			num_extra_chunks,
			num_extra_css_chunks,
			global_env_text,
			web_js_url,
			extra_chunks,
			extra_css_chunks,
		})
	}

	fn spawn_extra_js_chunk_tasks(&mut self) {
		for extra_chunk in mem::take(&mut self.extra_chunks) {
			self.spawn_js_scrape_task(extra_chunk);
		}
	}

	fn spawn_extra_css_chunk_tasks(&mut self) {
		for extra_chunk in mem::take(&mut self.extra_css_chunks) {
			self.spawn_css_chunk_task(extra_chunk);
		}
	}

	fn spawn_main_chunk_tasks(&mut self, tasks: Vec<ChunkHashEntry>) {
		for entry in tasks {
			self.spawn_js_scrape_task(entry);
		}
	}

	fn spawn_js_scrape_task(&mut self, entry: ChunkHashEntry) {
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
				if RUNTIME_FINDER.find(&chunk_bts).is_some() {
					trace!("Skipping worker chunk");
					inner.progress.chunk_finished();
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
					.map_err(|e| {
						anyhow!(
							"{:?}",
							e.with_local_source(chunk_str, &chunk_name)
						)
					})
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
	fn spawn_css_chunk_tasks(&mut self, chunks: Vec<ChunkHashEntry>) {
		for chunk in chunks {
			self.spawn_css_chunk_task(chunk);
		}
	}

	fn spawn_css_chunk_task(&mut self, chunk: ChunkHashEntry) {
		let hash = chunk.hash;
		let inner = self.inner.clone();
		self.chunk_futures.spawn(async move {
			let permit = inner
				.pending_limit
				.acquire()
				.await
				.unwrap();
			let chunk_name = format_smolstr!("{hash}.css");
			let chunk_url = asset_url(inner.channel, &chunk_name);
			let resp = inner
				.client
				.get(&chunk_url)
				.send()
				.await?;
			if resp.status() == StatusCode::NOT_FOUND {
				bail!("Chunk not found: {chunk_url}");
			}
			let css_bts = resp.bytes().await?;
			drop(permit);
			// TODO: css bytes total?
			let css_bts = Vec::from(css_bts);
			match String::from_utf8(css_bts) {
				Ok(s) => {
					inner.css_chunks.insert(chunk_name, s);
				}
				Err(FromUtf8Error { .. }) => {
					warn!("css chunk {chunk_url:?} is not valid utf8");
				}
			}
			inner.progress.chunk_finished();
			Ok(())
		});
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
		let src = main_js_text.as_ref();
		let main_parser = WebpackMainChunkParser::try_new(alloc, src)
			.map_err(|e| {
				anyhow!("{:?}", e.with_local_source(src, &self.web_js_url))
			})
			.context("Failed to parse main js chunk")?;
		let js_chunks = main_parser
			.get_js_chunk_hashes()
			.map_err(|e| {
				anyhow!("{:?}", e.with_local_source(src, &self.web_js_url))
			})
			.context("Failed to get js chunk hashes")?;
		let css_chunks = main_parser
			.get_css_chunk_hashes()
			.map_err(|e| {
				anyhow!("{:?}", e.with_local_source(src, &self.web_js_url))
			})
			.context("Failed to get css chunk hashes")?;
		let num_js_chunks = js_chunks.len() + self.num_extra_chunks;
		let num_css_chunks = css_chunks.len() + self.num_extra_css_chunks;
		let num_chunks = num_js_chunks + num_css_chunks;
		debug!("found {num_js_chunks} js chunks, {num_css_chunks} css chunks");
		self.inner.progress.set_chunk_total(num_chunks);
		self.spawn_main_chunk_tasks(js_chunks);
		self.spawn_css_chunk_tasks(css_chunks);
		let build_number = main_parser
			.get_build_number()
			.map_err(|e| e.with_local_source(src, &self.web_js_url))
			.inspect_err(|e| warn!("Failed to get the build number: {e:?}"))
			.unwrap_or_default()
			.parse()
			.unwrap_or_default();
		let entry_point = main_parser
			.get_entrypoint_id()
			.map_err(|e| e.with_local_source(src, &self.web_js_url))
			.inspect_err(|e| warn!("Failed to get the entrypoint id: {e:?}"))
			.ok();
		let main_modules = main_parser
			.collect_defined_modules()
			.map_err(|e| {
				anyhow!("{:?}", e.with_local_source(src, &self.web_js_url))
			})
			.context("Failed to get modules from main chunk")?;
		let size_hint = main_modules.size_hint();
		let mut keys = Vec::with_capacity(size_hint.1.unwrap_or(size_hint.0));
		for (m_id, src) in main_modules {
			self.inner.modules.insert(m_id, src);
			keys.push(m_id);
		}

		self.inner
			.module_sources
			.insert(self.web_js_url.clone(), keys);

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
		scraper.spawn_extra_js_chunk_tasks();
		scraper.spawn_extra_css_chunk_tasks();

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
		let css_chunks = inner.css_chunks;

		Ok(ScrapedModules {
			modules: HashMap::from_iter(modules),
			module_sources: HashMap::from_iter(module_sources),
			global_env_text: scraper.global_env_text,
			web_js_url: scraper.web_js_url,
			build_number,
			entry_point,
			css_chunks: HashMap::from_iter(css_chunks),
		})
	}
}

/// A freshly scraped build, along with the intl messages stored next to it
pub struct ScrapedBundle {
	pub bundle: FullBundle,
	pub intl: IntlMessages,
}

pub async fn scrape_full_bundle(
	html: &str,
	channel: Channel,
	build_hash: String,
	client: Arc<ClientWithMiddleware>,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<ScrapedBundle> {
	let ScrapedModules {
		modules,
		module_sources,
		global_env_text,
		web_js_url: _,
		build_number,
		entry_point,
		css_chunks,
	} = JsScraper::scrape(html, channel, client, progress.clone()).await?;

	let parsed = ParsedBundle::new(&modules, progress)?;
	let experiments = parsed
		.find_experiments()
		.await
		.inspect_err(|e| warn!("Failed to collect experiments: {e:?}"))
		.unwrap_or_default();
	let intl = parsed
		.find_intl()
		.await
		.inspect_err(|e| warn!("Failed to collect intl messages: {e:?}"))
		.unwrap_or_default();
	let dep_info = parsed.into_dep_info();

	let current_time = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("Bad System Clock")?
		.as_millis();
	debug_assert!(u64::try_from(current_time).is_ok());
	let first_seen = current_time as u64;

	let bundle = FullBundle {
		metadata: BundleMetadata {
			build_hash,
			build_number,
			first_seen,
			entry_point,
			channel: vec![channel],
		},
		dep_info,
		module_sources,
		modules,
		env_var_text: global_env_text,
		experiments,
		css_chunks,
	};
	Ok(ScrapedBundle { bundle, intl })
}
