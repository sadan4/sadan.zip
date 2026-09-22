mod disk;

use std::{
	collections::HashMap,
	env,
	ffi::OsStr,
	fmt::Write as _,
	io::ErrorKind,
	path::{Path, PathBuf},
	sync::{Arc, Mutex, PoisonError, Weak},
	thread::{self, ThreadId},
	time::{Instant, SystemTime},
};

use anyhow::{Context, Result};
use ast_parser::pool::AllocPool;
use async_trait::async_trait;
use dashmap::DashMap;
use explorer_types::{IncomingModuleDeps, ModuleId};
use pretty_printer::format_with_alloc;
use tokio::{
	fs,
	sync::{OnceCell, RwLock},
	task,
};
use tower_lsp_server::{
	Client,
	ls_types::{Uri, WorkDoneProgressBegin},
};
use tracing::{debug, info, instrument, warn};
use url::Url;
use webpack_ast_parser::{
	ThreadSafeParser,
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

use crate::{
	SERVER_VERSION,
	lsp::{
		self,
		custom::{Ephemera, EphemeralDocument, ephemera},
		doc,
	},
	module_cache::disk::{
		CACHE_FILE_NAME,
		CachedDepGraph,
		ModuleRootFingerprint,
	},
	util::err::display_no_backtrace,
	wss::{
		self,
		types::to_client::{ExtractMessage, FindQuery},
	},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeadlockId {
	Task(task::Id),
	Thread(ThreadId),
}

impl DeadlockId {
	fn current() -> Self {
		if let Some(id) = task::try_id() {
			Self::Task(id)
		} else {
			Self::Thread(thread::current_id())
		}
	}
}

pub struct SplitModuleCache {
	disk: Arc<DiskModuleCache>,
	live: Arc<LiveModuleCache>,
	/// One per open document that holds its own copy of a module; see
	/// [`BufferModuleCache`].
	buffers: DashMap<Uri, Arc<BufferModuleCache>>,
	files: doc::Files,
}

struct DiskModuleCache {
	parsers: DashMap<ModuleId, Arc<ThreadSafeParser>>,
	/// The dep graph, once it has been built.
	dep_graph: RwLock<Option<CachedDepGraph>>,
	/// The task building [`Self::dep_graph`], if it is currently being built.
	dep_graph_builder: Mutex<Option<DeadlockId>>,
	/// Where to look for [`MODULE_DIR_NAME`], best candidate first.
	workspace_roots: OnceCell<Vec<PathBuf>>,
	/// The resolved module directory, once one has been found.
	module_root: Mutex<Option<PathBuf>>,
	client: OnceCell<Client>,
	this: Weak<Self>,
}

/// The directory modules are dumped into, relative to a workspace root.
const MODULE_DIR_NAME: &str = ".modules";

/// The directory live modules are published under, in the ephemeral document
/// tree.
const LIVE_MODULE_DIR: &str = "/modules";

/// The indent live modules are formatted with; `0` means tabs.
const INDENT: u8 = 2;

/// Whether a live module is fetched with the patches the client has applied.
// TODO: user setting, matching what the module dump asks for
const USE_PATCHED: bool = false;

/// Where the live copy of `id` is published.
///
/// The file stem is the bare id so that [`module_id_from_uri`] can read it
/// back off the URI the client sends us.
fn live_module_uri(id: ModuleId) -> Result<Uri> {
	ephemera::uri(format!("{LIVE_MODULE_DIR}/{id}.js"))
}

/// [`live_module_uri`] as a [`Url`], which is what [`IModuleCache`] hands back.
fn live_module_url(id: ModuleId) -> Result<Url> {
	let uri = live_module_uri(id)?;
	Url::parse(uri.as_str()).context("Live module URI is not a valid URL")
}

/// Whether `uri` is one of the documents the live cache publishes.
///
/// The `vencord-companion` scheme carries more than live modules — patch
/// helper views live under it too — so the scheme alone does not say which
/// cache owns a document.
fn is_live_module_uri(uri: &Uri) -> bool {
	Path::new(uri.path().as_str()).parent() == Some(Path::new(LIVE_MODULE_DIR))
		&& module_id_from_uri(uri).is_some()
}

/// The module a uri points at, taken from its file name the same way
/// [`DiskModuleCache::scan_module_root`] does.
fn module_id_from_uri(uri: &Uri) -> Option<ModuleId> {
	let path = Path::new(uri.path().as_str());
	if path.extension() != Some(OsStr::new("js")) {
		return None;
	}
	path.file_stem()?
		.to_str()?
		.parse()
		.ok()
		.map(ModuleId)
}

/// Marks the current task as the one building the dep graph, for as long as it
/// is alive.
///
/// Building the graph parses modules, and parsing is what asks for the graph,
/// so a build that reaches back into [`DiskModuleCache::get_module_deps`] would
/// wait on the [`OnceCell`] it is itself initializing. That is an unrecoverable
/// deadlock, so the guard lets the re-entrant call panic instead.
struct DepGraphBuildGuard<'a> {
	builder: &'a Mutex<Option<DeadlockId>>,
}

impl Drop for DepGraphBuildGuard<'_> {
	fn drop(&mut self) {
		// this also runs while panicking, so do not re-panic on a poisoned lock
		*self
			.builder
			.lock()
			.unwrap_or_else(PoisonError::into_inner) = None;
	}
}

struct LiveModuleCache {
	socket: wss::WsServer,
	parsers: DashMap<ModuleId, Arc<ThreadSafeParser>>,
	/// The disk cache, for [`IModuleDepProvider`].
	///
	/// Nothing on the wire answers "what requires this module", so incoming
	/// deps come from the dumped graph. Webpack ids are deterministic, so a
	/// dump from an earlier session is usually still right.
	disk: Arc<DiskModuleCache>,
	/// Where a fetched module is published for the client to open.
	ephemera: Arc<Ephemera>,
	pool: Arc<AllocPool>,
	this: Weak<Self>,
}

#[async_trait]
pub trait SplitCache: IModuleCache + IModuleDepProvider + Send + Sync {
	async fn get_parser(&self, id: ModuleId) -> Result<Arc<ThreadSafeParser>>;
	/// Drop everything cached for `id`, because its source changed under us.
	async fn invalidate(&self, id: ModuleId);
}

impl SplitModuleCache {
	pub fn new(
		socket: wss::WsServer,
		ephemera: Arc<Ephemera>,
		pool: Arc<AllocPool>,
		files: doc::Files,
	) -> Self {
		// TODO: setting or smth for module_root
		let disk = DiskModuleCache::new_arc();
		let live =
			LiveModuleCache::new_arc(socket, Arc::clone(&disk), ephemera, pool);
		Self {
			disk,
			live,
			buffers: DashMap::new(),
			files,
		}
	}

	pub fn set_workspace_roots(&self, roots: Vec<PathBuf>) {
		self.disk.set_workspace_roots(roots);
	}

	/// Drop what is cached for a module whose file was just written.
	pub async fn handle_save(&self, uri: &Uri) {
		let Some(id) = module_id_from_uri(uri) else {
			debug!(uri =% uri.as_str(), "Saved file is not a module, nothing to invalidate");
			return;
		};
		self.get_for_uri(uri)
			.invalidate(id)
			.await;
	}
	/// Drop the cache a closed document had.
	pub fn handle_close(&self, uri: &Uri) {
		if self.buffers.remove(uri).is_some() {
			debug!(uri =% uri.as_str(), "Dropped the buffer module cache");
		}
	}

	/// The cache that owns `uri`'s module.
	///
	/// A `vencord-companion` document is either a module the live cache
	/// published or a buffer holding its own copy of one, such as a patch
	/// helper view, so the scheme alone does not decide.
	pub fn get_for_uri(&self, uri: &Uri) -> Arc<dyn SplitCache> {
		match uri.scheme().as_str() {
			"file" => Arc::clone(&self.disk) as Arc<dyn SplitCache>,
			"vencord-companion" if is_live_module_uri(uri) => {
				Arc::clone(&self.live) as Arc<dyn SplitCache>
			}
			"vencord-companion" => self.buffer_cache(uri),
			scheme => {
				warn!(
					?scheme,
					"Unknown scheme for module cache, defaulting to disk"
				);
				Arc::clone(&self.disk) as Arc<dyn SplitCache>
			}
		}
	}

	/// The cache for `uri`'s own text, created on first use.
	fn buffer_cache(&self, uri: &Uri) -> Arc<dyn SplitCache> {
		let cache = Arc::clone(
			self.buffers
				.entry(uri.clone())
				.or_insert_with(|| {
					BufferModuleCache::new_arc(
						uri.clone(),
						self.files.clone(),
						Arc::clone(&self.live),
					)
				})
				.value(),
		);
		cache as Arc<dyn SplitCache>
	}
	pub fn set_client(&self, client: Client) {
		self.disk
			.client
			.set(client)
			.expect("client already set");
	}

	/// Delete the dumped modules and drop everything cached from them.
	///
	/// Returns the directory that was removed.
	pub async fn clear(&self) -> Result<PathBuf> {
		self.disk.clear().await
	}

	/// Where a fresh dump of the modules should go.
	pub fn module_dir_target(&self) -> Result<PathBuf> {
		self.disk.module_dir_target()
	}

	/// Drop every module fetched from the running client.
	///
	/// Returns how many were dropped. Nothing on disk is touched; that is
	/// [`Self::clear`]'s job.
	pub fn clear_live(&self) -> usize {
		self.live.clear()
	}
}

impl DiskModuleCache {
	fn new_arc() -> Arc<Self> {
		Arc::new_cyclic(|this| Self {
			parsers: DashMap::new(),
			dep_graph: RwLock::new(None),
			dep_graph_builder: Mutex::new(None),
			workspace_roots: OnceCell::new(),
			module_root: Mutex::new(None),
			client: OnceCell::new(),
			this: this.clone(),
		})
	}

	/// Record where the client says the workspace lives.
	fn set_workspace_roots(&self, roots: Vec<PathBuf>) {
		if self.workspace_roots.set(roots).is_err() {
			warn!("workspace roots already set, ignoring");
		}
	}

	/// The module directory, if one exists right now.
	///
	/// Resolved lazily, and re-resolved until it is found: the workspace roots
	/// only arrive with the `initialize` request, and the user may well dump
	/// modules after the server has already started. Only a hit is cached.
	async fn module_root(&self) -> Option<PathBuf> {
		let cached = self
			.module_root
			.lock()
			.unwrap_or_else(PoisonError::into_inner)
			.clone();
		if let Some(root) = cached {
			return Some(root);
		}
		let mut candidates = self
			.workspace_roots
			.get()
			.cloned()
			.unwrap_or_default();
		// the cwd is a guess, so it goes last
		match env::current_dir() {
			Ok(cwd) => candidates.push(cwd),
			Err(e) => warn!("Failed to get current working directory: {e}"),
		}
		for mut candidate in candidates {
			candidate.push(MODULE_DIR_NAME);
			if fs::try_exists(&candidate)
				.await
				.unwrap_or(false)
			{
				info!(path =% candidate.display(), "Found module root");
				*self
					.module_root
					.lock()
					.unwrap_or_else(PoisonError::into_inner) = Some(candidate.clone());
				return Some(candidate);
			}
		}
		None
	}

	/// Where a fresh dump of the modules should go: the module root if one is
	/// already resolved, otherwise [`MODULE_DIR_NAME`] under the first
	/// workspace root the client gave us, falling back to the cwd.
	fn module_dir_target(&self) -> Result<PathBuf> {
		let resolved = self
			.module_root
			.lock()
			.unwrap_or_else(PoisonError::into_inner)
			.clone();
		if let Some(root) = resolved {
			return Ok(root);
		}
		let mut root = match self
			.workspace_roots
			.get()
			.and_then(|roots| roots.first().cloned())
		{
			Some(root) => root,
			None => env::current_dir()
				.context("Failed to get the current working directory")?,
		};
		root.push(MODULE_DIR_NAME);
		Ok(root)
	}

	/// Delete the module root and drop everything cached from it.
	///
	/// Returns the directory that was removed.
	async fn clear(&self) -> Result<PathBuf> {
		let module_root = self
			.module_root()
			.await
			.context("No cache to clear")?;
		// held across the delete so a build in flight cannot re-fill the graph
		// from files that are about to go away
		let mut dep_graph = self.dep_graph.write().await;
		fs::remove_dir_all(&module_root)
			.await
			.with_context(|| {
				format!(
					"Failed to remove the module root at {}",
					module_root.display()
				)
			})?;
		*dep_graph = None;
		self.parsers.clear();
		// the directory is gone, so look for one again from scratch
		*self
			.module_root
			.lock()
			.unwrap_or_else(PoisonError::into_inner) = None;
		info!(path =% module_root.display(), "Cleared the module cache");
		Ok(module_root)
	}

	/// Get the client.
	fn client(&self) -> &Client {
		self.client
			.get()
			.expect("client not set")
	}
	/// Claim this task as the dep graph's builder.
	///
	/// Panics if this task is already building the graph; see
	/// [`DepGraphBuildGuard`].
	fn mark_building_dep_graph(&self) -> DepGraphBuildGuard<'_> {
		let tid = DeadlockId::current();
		let mut builder = self
			.dep_graph_builder
			.lock()
			.unwrap_or_else(PoisonError::into_inner);
		assert!(
			*builder != Some(tid),
			"re-entered the dep graph build from the task building it"
		);
		*builder = Some(tid);
		drop(builder);
		DepGraphBuildGuard {
			builder: &self.dep_graph_builder,
		}
	}

	/// Whether the current task is the one building the dep graph.
	fn is_building_dep_graph(&self) -> bool {
		let tid = DeadlockId::current();
		*self
			.dep_graph_builder
			.lock()
			.unwrap_or_else(PoisonError::into_inner)
			== Some(tid)
	}

	/// Read the dep graph from disk, building (and writing) it if there is no
	/// usable cache.
	async fn load_dep_graph(&self) -> Result<CachedDepGraph> {
		let _guard = self.mark_building_dep_graph();
		let module_root = self
			.module_root()
			.await
			.context("No module root, cannot build a dep graph")?;
		let module_root = module_root.as_path();
		let (ids, fingerprint) = Self::scan_module_root(module_root).await?;
		let cache_path = module_root.join(CACHE_FILE_NAME);
		match fs::read(&cache_path).await {
			Ok(contents) => {
				match serde_json::from_slice::<CachedDepGraph>(&contents) {
					Ok(graph)
						if graph.version == SERVER_VERSION
							&& graph.fingerprint == fingerprint =>
					{
						debug!(
							modules = graph.inverse_deps.len(),
							"Loaded dep graph from disk"
						);
						return Ok(graph);
					}
					Ok(graph) if graph.version != SERVER_VERSION => {
						info!(
							cached =% graph.version,
							current = SERVER_VERSION,
							"Dep graph cache is from another version, rebuilding"
						);
					}
					Ok(_) => {
						info!(
							"Modules have changed since the dep graph was cached, rebuilding"
						);
					}
					Err(e) => {
						warn!(
							path =% cache_path.display(),
							"Failed to parse dep graph cache, rebuilding: {e}"
						);
					}
				}
			}
			Err(e) if e.kind() == ErrorKind::NotFound => {
				debug!("No dep graph cache on disk, building one");
			}
			Err(e) => {
				warn!(
					path =% cache_path.display(),
					"Failed to read dep graph cache, rebuilding: {e}"
				);
			}
		}
		let start = Instant::now();
		let graph = self
			.build_dep_graph(&ids, fingerprint)
			.await?;
		info!(
			modules = graph.inverse_deps.len(),
			elapsed =? start.elapsed(),
			"Built dep graph"
		);
		// a graph we could not write is still usable
		match serde_json::to_vec(&graph) {
			Ok(contents) => {
				if let Err(e) = fs::write(&cache_path, contents).await {
					warn!(
						path =% cache_path.display(),
						"Failed to write dep graph cache: {e}"
					);
				}
			}
			Err(e) => warn!("Failed to serialize dep graph cache: {e}"),
		}
		Ok(graph)
	}

	/// Parse every module in `ids` and invert their dependencies.
	///
	/// Modules that fail to read or parse are skipped; they just do not
	/// contribute any dependents.
	///
	/// This may only use parts of the parsers that do not themselves need the
	/// dep graph, or it would re-enter [`Self::dep_graph`]'s initialization and
	/// crash.
	async fn build_dep_graph(
		&self,
		ids: &[ModuleId],
		fingerprint: ModuleRootFingerprint,
	) -> Result<CachedDepGraph> {
		let handle = lsp::Server::start_progress(
			self.client().clone(),
			WorkDoneProgressBegin {
				title: "Building dep graph".to_string(),
				..Default::default()
			},
		);
		let mut inverse_deps: HashMap<ModuleId, IncomingModuleDeps> =
			HashMap::with_capacity(ids.len());
		let mut cur = 0;
		let total = ids.len();
		for id in ids {
			cur += 1;
			handle.step(
				(cur as f32 * 100.0 / total as f32).floor() as u32,
				format!("Parsing module {id} ({cur}/{total})"),
			);
			let parser = match self.get_parser(*id).await {
				Ok(parser) => parser,
				Err(e) => {
					// avoid massive backtrace for spurious errors
					let e = display_no_backtrace(&e);
					warn!(%id, "Failed to get parser for module: {e}");
					continue;
				}
			};
			let Some(outgoing) = parser
				.parser()
				.get_modules_that_this_module_requires()
			else {
				continue;
			};
			for dep in &outgoing.sync {
				inverse_deps
					.entry(dep.id)
					.or_default()
					.sync
					.push(*id);
			}
			for dep in &outgoing.lazy {
				inverse_deps
					.entry(dep.id)
					.or_default()
					.lazy
					.push(*id);
			}
		}
		debug!(
			modules = ids.len(),
			dependents = inverse_deps.len(),
			"Built dep graph"
		);
		Ok(CachedDepGraph {
			version: String::from(SERVER_VERSION),
			fingerprint,
			inverse_deps,
		})
	}

	/// The ids of every module in `module_root`, taken from the file names,
	/// along with a fingerprint of what was found.
	async fn scan_module_root(
		module_root: &Path,
	) -> Result<(Vec<ModuleId>, ModuleRootFingerprint)> {
		let mut dir = fs::read_dir(module_root)
			.await
			.context("Failed to read the module root")?;
		let mut ids = Vec::new();
		let mut fingerprint = ModuleRootFingerprint::default();
		while let Some(entry) = dir
			.next_entry()
			.await
			.context("Failed to read the module root")?
		{
			let path = entry.path();
			if path.extension() != Some(OsStr::new("js")) {
				continue;
			}
			let Some(id) = path
				.file_stem()
				.and_then(OsStr::to_str)
				.and_then(|stem| stem.parse().ok())
				.map(ModuleId)
			else {
				warn!(
					path =% path.display(),
					"Module file is not named after a module id, skipping"
				);
				continue;
			};
			// a module we cannot stat still counts, it just cannot move the
			// mtime; worst case the fingerprint misses a change
			let mtime_ms = entry
				.metadata()
				.await
				.and_then(|meta| meta.modified())
				.ok()
				.and_then(|mtime| {
					mtime
						.duration_since(SystemTime::UNIX_EPOCH)
						.ok()
				})
				.map_or(0, |since_epoch| since_epoch.as_millis());
			fingerprint.add_module(mtime_ms);
			ids.push(id);
		}
		Ok((ids, fingerprint))
	}

	async fn module_path(&self, id: ModuleId) -> Option<PathBuf> {
		let mut path = self.module_root().await?;
		// module id 6 + ext 3 + `/` 1
		path.reserve(10);
		path.set_trailing_sep(true);
		write!(path.as_mut_os_string(), "{id}.js").unwrap();
		let path = fs::canonicalize(&path)
			.await
			.inspect_err(|e| {
				warn!(path =% path.display(), "Failed to canonicalize module file: {e}");
			})
			.ok()?;
		match fs::try_exists(&path).await {
			Ok(e) => e.then_some(path),
			Err(e) => {
				warn!(path =% path.display(), "Failed to check existance of module file: {e}");
				None
			}
		}
	}
}

#[async_trait]
impl SplitCache for DiskModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn invalidate(&self, id: ModuleId) {
		// the next `get_parser` re-reads the file
		if self.parsers.remove(&id).is_none() {
			debug!(%id, "Nothing cached for module, nothing to invalidate");
			return;
		}
		debug!(%id, "Dropped cached parser for module");
		// nothing to patch if the graph has not been built yet, and rebuilding
		// it here would parse every module for a change to one of them
		if self.dep_graph.read().await.is_none() {
			return;
		}
		// what the module requires can have changed, so re-invert just its edges
		let parser = self
			.get_parser(id)
			.await
			.inspect_err(|e| {
				let e = display_no_backtrace(e);
				warn!(%id, "Failed to re-parse changed module: {e}");
			})
			.ok();
		let outgoing = parser.as_ref().and_then(|parser| {
			parser
				.parser()
				.get_modules_that_this_module_requires()
		});
		let mut dep_graph = self.dep_graph.write().await;
		let Some(dep_graph) = dep_graph.as_mut() else {
			return;
		};
		for deps in dep_graph.inverse_deps.values_mut() {
			deps.sync.retain(|dep| *dep != id);
			deps.lazy.retain(|dep| *dep != id);
		}
		let Some(outgoing) = outgoing else {
			return;
		};
		for dep in &outgoing.sync {
			dep_graph
				.inverse_deps
				.entry(dep.id)
				.or_default()
				.sync
				.push(id);
		}
		for dep in &outgoing.lazy {
			dep_graph
				.inverse_deps
				.entry(dep.id)
				.or_default()
				.lazy
				.push(id);
		}
	}

	#[instrument(skip_all, fields(id))]
	async fn get_parser(&self, id: ModuleId) -> Result<Arc<ThreadSafeParser>> {
		let parser = if let Some(parser) = self.parsers.get(&id) {
			Arc::clone(&parser)
		} else {
			let file_path = self
				.module_path(id)
				.await
				.context("Failed to get module filepath")?;
			let contents = fs::read_to_string(file_path)
				.await
				.context("Failed to read module file")?;
			let mut parser = ThreadSafeParser::new(Arc::from(contents))
				.context("Failed to parse module")?;
			let this = Arc::new(WeakCache(self.this.clone()));
			parser.set_module_cache(this.clone());
			parser.set_module_dep_provider(this);
			let parser = Arc::new(parser);
			self.parsers
				.insert(id, Arc::clone(&parser));
			parser
		};
		Ok(parser)
	}
}

#[async_trait]
impl IModuleDepProvider for DiskModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn get_module_deps(
		&self,
		id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		// the builder holds the lock for the whole build, so waiting on it from
		// inside that build never finishes
		assert!(
			!self.is_building_dep_graph(),
			"asked for the dep graph while building it"
		);
		let dep_graph = self.dep_graph.read().await;
		if let Some(dep_graph) = dep_graph.as_ref() {
			Ok(Arc::new(
				dep_graph
					.inverse_deps
					.get(&id)
					.cloned()
					.unwrap_or_default(),
			))
		} else {
			drop(dep_graph);
			let mut dep_graph = self.dep_graph.write().await;
			Ok(Arc::new(
				dep_graph
					.insert(self.load_dep_graph().await?)
					.inverse_deps
					.get(&id)
					.cloned()
					.unwrap_or_default(),
			))
		}
		// a module nothing requires has no entry in the graph
	}
}

#[async_trait]
impl IModuleCache for DiskModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn get_module_filepath(&self, id: ModuleId) -> Option<url::Url> {
		self.module_path(id).await.map(|path| {
			url::Url::from_file_path(path).expect("path is not canonical")
		})
	}

	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> anyhow::Result<Arc<ThreadSafeParser>> {
		self.get_parser(id).await
	}
}

/// The handle a cached parser gets back to the cache holding it.
///
/// Weak so that the cache -> parser -> cache path is not a reference cycle
struct WeakCache<T>(Weak<T>);

impl<T> WeakCache<T> {
	fn get(&self) -> Result<Arc<T>> {
		self.0
			.upgrade()
			.context("Module cache has been dropped")
	}
}

#[async_trait]
impl<T: IModuleDepProvider + 'static> IModuleDepProvider for WeakCache<T> {
	async fn get_module_deps(
		&self,
		id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		self.get()?.get_module_deps(id).await
	}
}

#[async_trait]
impl<T: IModuleCache + 'static> IModuleCache for WeakCache<T> {
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url> {
		match self.get() {
			Ok(cache) => cache.get_module_filepath(id).await,
			Err(e) => {
				warn!("{e}");
				None
			}
		}
	}

	async fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		self.get()?
			.get_module_parser(requestor, id, latest)
			.await
	}
}

/// What a live module fetch fails with when there is no dep graph to fall back
/// on.
const NO_DEP_GRAPH: &str = "live modules take their dependents from the module \
                            dump; run the Download Module Cache command to \
                            build one";

impl LiveModuleCache {
	fn new_arc(
		socket: wss::WsServer,
		disk: Arc<DiskModuleCache>,
		ephemera: Arc<Ephemera>,
		pool: Arc<AllocPool>,
	) -> Arc<Self> {
		Arc::new_cyclic(|this| Self {
			socket,
			parsers: DashMap::new(),
			disk,
			ephemera,
			pool,
			this: this.clone(),
		})
	}

	/// Ask the client for module `id`, formatted the same way the dumped
	/// modules are.
	///
	/// The header has to go on before the pretty printer runs, or the result
	/// does not parse as a webpack module and nothing downstream can find its
	/// id.
	async fn fetch_module(&self, id: ModuleId) -> Result<Arc<str>> {
		let res = self
			.socket
			.send_msg(ExtractMessage {
				data: FindQuery::Id {
					id,
					use_patched: USE_PATCHED,
				},
			})
			.await
			.with_context(|| {
				format!("Failed to extract module {id} from the client")
			})?;
		let got = res.module_result.module_number;
		if got != id {
			warn!(%id, %got, "Client answered with a different module than we asked for");
		}
		let mut src = res.module;
		// cpu-bound AST work; block_in_place keeps it off the tokio worker
		let src = task::block_in_place(|| {
			WebpackAstParser::format_module_header(&mut src, id, false);
			let alloc = self.pool.get();
			// an unformatted module parses just as well as a formatted one
			match format_with_alloc(&src, &alloc, INDENT) {
				Ok(content) => content.code,
				Err(e) => {
					warn!(%id, "Failed to format live module, using it as-is: {e}");
					src
				}
			}
		});
		Ok(Arc::from(src))
	}

	/// Drop every module fetched so far, along with the documents publishing
	/// them.
	///
	/// Returns how many were dropped.
	fn clear(&self) -> usize {
		let ids: Vec<ModuleId> = self
			.parsers
			.iter()
			.map(|entry| *entry.key())
			.collect();
		self.parsers.clear();
		for id in &ids {
			match live_module_uri(*id) {
				// a document the client already dropped is not a problem
				Ok(uri) => {
					if let Err(e) = self.ephemera.delete(uri) {
						debug!(%id, "Nothing to delete for live module: {e}");
					}
				}
				Err(e) => {
					warn!(%id, "Failed to build live module URI: {e}");
				}
			}
		}
		info!(modules = ids.len(), "Cleared the live module cache");
		ids.len()
	}
}

#[async_trait]
impl SplitCache for LiveModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn get_parser(&self, id: ModuleId) -> Result<Arc<ThreadSafeParser>> {
		if let Some(parser) = self.parsers.get(&id) {
			return Ok(Arc::clone(&parser));
		}
		let src = self.fetch_module(id).await?;
		let mut parser = ThreadSafeParser::new(Arc::clone(&src))
			.context("Failed to parse module")?;
		let this = Arc::new(WeakCache(self.this.clone()));
		parser.set_module_cache(this.clone());
		parser.set_module_dep_provider(this);
		// the published text has to be byte for byte what the parser holds, or
		// `lsp::cursor_offset` rejects every position in the document
		self.ephemera.upsert(EphemeralDocument {
			uri: live_module_uri(id)?,
			content: src,
		});
		// whoever got here first keeps the parser it already handed out
		let parser = Arc::clone(
			self.parsers
				.entry(id)
				.or_insert(Arc::new(parser))
				.value(),
		);
		debug!(%id, "Fetched live module from the client");
		Ok(parser)
	}

	async fn invalidate(&self, id: ModuleId) {
		// the published document is left alone: the user may have it open, and
		// the next `get_parser` republishes it anyway
		if self.parsers.remove(&id).is_none() {
			debug!(%id, "Nothing cached for live module, nothing to invalidate");
			return;
		}
		debug!(%id, "Dropped cached parser for live module");
	}
}

#[async_trait]
impl IModuleDepProvider for LiveModuleCache {
	async fn get_module_deps(
		&self,
		id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		self.disk
			.get_module_deps(id)
			.await
			.context(NO_DEP_GRAPH)
	}
}

#[async_trait]
impl IModuleCache for LiveModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url> {
		// fetching publishes the document, so the URI we hand back is one the
		// client can actually open
		if let Err(e) = self.get_parser(id).await {
			let e = display_no_backtrace(&e);
			warn!(%id, "Failed to fetch live module: {e}");
			return None;
		}
		live_module_url(id)
			.inspect_err(|e| warn!(%id, "Failed to build live module URL: {e}"))
			.ok()
	}

	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		self.get_parser(id).await
	}
}

/// A document holding its own copy of a module, such as a patch helper view.
///
/// The text in front of the user *is* the module, so it is parsed from the
/// buffer. A copy fetched by id would be the unpatched original: it would not
/// match the open document, and every cursor position in it would be rejected
/// by [`lsp::cursor_offset`].
struct BufferModuleCache {
	uri: Uri,
	files: doc::Files,
	/// The modules this buffer is not; a definition or reference in it can
	/// point at any of them.
	live: Arc<LiveModuleCache>,
	parsed: Mutex<Option<ParsedBuffer>>,
	this: Weak<Self>,
}

/// A parsed buffer, along with the text it was parsed from so an edited buffer
/// is noticed.
struct ParsedBuffer {
	id: ModuleId,
	source: Arc<str>,
	parser: Arc<ThreadSafeParser>,
}

impl BufferModuleCache {
	fn new_arc(
		uri: Uri,
		files: doc::Files,
		live: Arc<LiveModuleCache>,
	) -> Arc<Self> {
		Arc::new_cyclic(|this| Self {
			uri,
			files,
			live,
			parsed: Mutex::new(None),
			this: this.clone(),
		})
	}

	/// The parser for this document's own text, reparsed if the buffer has
	/// changed since the last one was built.
	fn own(&self) -> Result<(ModuleId, Arc<ThreadSafeParser>)> {
		let doc = self
			.files
			.get(&self.uri)
			.context("Document is not open")?;
		{
			let parsed = self
				.parsed
				.lock()
				.unwrap_or_else(PoisonError::into_inner);
			if let Some(parsed) = parsed.as_ref()
				&& &*parsed.source == doc.text.as_str()
			{
				return Ok((parsed.id, Arc::clone(&parsed.parser)));
			}
		}
		let id = WebpackAstParser::parse_module_id(&doc.text)
			.context("Document does not start with a webpack module header")?
			.id;
		let source: Arc<str> = Arc::from(doc.text.as_str());
		// cpu-bound; block_in_place keeps it off the tokio worker
		let mut parser =
			task::block_in_place(|| ThreadSafeParser::new(Arc::clone(&source)))
				.context("Failed to parse the document")?;
		let this = Arc::new(WeakCache(self.this.clone()));
		parser.set_module_cache(this.clone());
		parser.set_module_dep_provider(this);
		let parser = Arc::new(parser);
		*self
			.parsed
			.lock()
			.unwrap_or_else(PoisonError::into_inner) = Some(ParsedBuffer {
			id,
			source,
			parser: Arc::clone(&parser),
		});
		debug!(uri =% self.uri.as_str(), %id, "Parsed a module out of its document");
		Ok((id, parser))
	}

	/// This document's own module id, if it holds one.
	fn own_id(&self) -> Option<ModuleId> {
		self.own()
			.inspect_err(|e| {
				debug!(uri =% self.uri.as_str(), "Document is not a module: {e}");
			})
			.ok()
			.map(|(id, _)| id)
	}
}

#[async_trait]
impl SplitCache for BufferModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn get_parser(&self, id: ModuleId) -> Result<Arc<ThreadSafeParser>> {
		match self.own() {
			Ok((own_id, parser)) if own_id == id => Ok(parser),
			// anything this buffer is not a copy of is a real module
			Ok(_) => self.live.get_parser(id).await,
			Err(e) => {
				debug!(%id, "Not serving module from the document: {e}");
				self.live.get_parser(id).await
			}
		}
	}

	async fn invalidate(&self, _: ModuleId) {
		// the buffer is the source of truth, and `own` reparses whenever its
		// text changes, so there is nothing to do but drop what we have
		*self
			.parsed
			.lock()
			.unwrap_or_else(PoisonError::into_inner) = None;
	}
}

#[async_trait]
impl IModuleDepProvider for BufferModuleCache {
	async fn get_module_deps(
		&self,
		id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		self.live.get_module_deps(id).await
	}
}

#[async_trait]
impl IModuleCache for BufferModuleCache {
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url> {
		// a reference back into this module belongs in the document the user
		// is looking at, not in a fresh copy of it
		if self.own_id() == Some(id) {
			return Url::parse(self.uri.as_str())
				.inspect_err(|e| {
					warn!(uri =% self.uri.as_str(), "Document URI is not a valid URL: {e}");
				})
				.ok();
		}
		self.live.get_module_filepath(id).await
	}

	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		self.get_parser(id).await
	}
}

#[cfg(test)]
mod tests {
	use explorer_types::ModuleId;

	use super::{
		ephemera,
		is_live_module_uri,
		live_module_uri,
		live_module_url,
		module_id_from_uri,
	};

	/// The URI a live module is published under is the one
	/// [`super::SplitModuleCache::get_for_uri`] routes back to the live cache,
	/// and the id has to survive the round trip.
	#[test]
	fn live_module_uris_round_trip() {
		let id = ModuleId(123);
		let uri = live_module_uri(id).unwrap();
		assert_eq!(uri.as_str(), "vencord-companion:/modules/123.js");
		assert_eq!(uri.scheme().as_str(), "vencord-companion");
		assert_eq!(module_id_from_uri(&uri), Some(id));
	}

	/// The `vencord-companion` scheme is shared with the patch helper, whose
	/// documents are their own copy of a module and must not be answered with
	/// a fresh one fetched by id.
	#[test]
	fn only_published_modules_are_live_module_uris() {
		assert!(is_live_module_uri(&live_module_uri(ModuleId(123)).unwrap()));
		for path in [
			"/patch-helper/plugins-MyPlugin.ts-0.js",
			// a patch helper view of a module could still be named after one
			"/patch-helper/123.js",
			"/modules/not-an-id.js",
			"/123.js",
		] {
			let uri = ephemera::uri(path).unwrap();
			assert!(!is_live_module_uri(&uri), "{path}");
		}
	}

	/// `vencord-companion:` is not a special scheme, so the path is opaque to
	/// the `url` crate; it still has to serialize back to what we built.
	#[test]
	fn live_module_urls_match_their_uris() {
		let id = ModuleId(123);
		assert_eq!(
			live_module_url(id).unwrap().as_str(),
			live_module_uri(id).unwrap().as_str()
		);
	}
}
