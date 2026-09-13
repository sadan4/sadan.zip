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

use anyhow::{Context, Result, bail};
use dashmap::DashMap;
use explorer_types::{IncomingModuleDeps, ModuleId};
use tokio::{
	fs,
	sync::{OnceCell, RwLock},
	task,
};
use tower_lsp::{Client, async_trait, lsp_types::WorkDoneProgressBegin};
use tracing::{debug, info, instrument, warn};
use url::Url;
use webpack_ast_parser::{
	ThreadSafeParser,
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

use crate::{
	SERVER_VERSION,
	lsp,
	module_cache::disk::{
		CACHE_FILE_NAME,
		CachedDepGraph,
		ModuleRootFingerprint,
	},
	util::err::display_no_backtrace,
	wss,
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

/// The module a uri points at, taken from its file name the same way
/// [`DiskModuleCache::scan_module_root`] does.
fn module_id_from_uri(uri: &Url) -> Option<ModuleId> {
	let path = Path::new(uri.path());
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

#[expect(unused)]
struct LiveModuleCache {
	socket: wss::WsServer,
	parsers: DashMap<ModuleId, Arc<ThreadSafeParser>>,
}

#[async_trait]
pub trait SplitCache: IModuleCache + IModuleDepProvider + Send + Sync {
	async fn get_parser(&self, id: ModuleId) -> Result<Arc<ThreadSafeParser>>;
	/// Drop everything cached for `id`, because its source changed under us.
	async fn invalidate(&self, id: ModuleId);
}

impl SplitModuleCache {
	pub fn new(socket: wss::WsServer) -> Self {
		// TODO: setting or smth for module_root
		let disk = DiskModuleCache::new_arc();
		let live = Arc::new(LiveModuleCache::new(socket));
		Self { disk, live }
	}

	pub fn set_workspace_roots(&self, roots: Vec<PathBuf>) {
		self.disk.set_workspace_roots(roots);
	}

	/// Drop what is cached for a module whose file was just written.
	pub async fn handle_save(&self, uri: &Url) {
		let Some(id) = module_id_from_uri(uri) else {
			debug!(%uri, "Saved file is not a module, nothing to invalidate");
			return;
		};
		self.get_for_uri(uri)
			.invalidate(id)
			.await;
	}
	pub fn get_for_uri(&self, uri: &Url) -> Arc<dyn SplitCache> {
		match uri.scheme() {
			"file" => Arc::clone(&self.disk) as Arc<dyn SplitCache>,
			"vencord-companion" => {
				Arc::clone(&self.live) as Arc<dyn SplitCache>
			}
			scheme => {
				warn!(
					scheme,
					"Unknown scheme for module cache, defaulting to disk"
				);
				Arc::clone(&self.disk) as Arc<dyn SplitCache>
			}
		}
	}
	pub fn set_client(&self, client: Client) {
		self.disk
			.client
			.set(client)
			.expect("client already set");
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
			let this = Arc::new(WeakDiskCache(self.this.clone()));
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
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url> {
		self.module_path(id).await.map(|path| {
			Url::from_file_path(path).expect("path is not canonical")
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

/// The handle a parser in [`DiskModuleCache::parsers`] gets back to the cache
/// holding it.
///
/// Weak so that the cache -> parser -> cache path is not a reference cycle
struct WeakDiskCache(Weak<DiskModuleCache>);

impl WeakDiskCache {
	fn get(&self) -> Result<Arc<DiskModuleCache>> {
		self.0
			.upgrade()
			.context("Module cache has been dropped")
	}
}

#[async_trait]
impl IModuleDepProvider for WeakDiskCache {
	async fn get_module_deps(
		&self,
		id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		self.get()?.get_module_deps(id).await
	}
}

#[async_trait]
impl IModuleCache for WeakDiskCache {
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

const UNIMPLEMENTED: &str = "the live module cache is not implemented yet; open the module from \
	 `.modules` on disk instead";

impl LiveModuleCache {
	fn new(socket: wss::WsServer) -> Self {
		Self {
			socket,
			parsers: DashMap::new(),
		}
	}
}

#[async_trait]
impl SplitCache for LiveModuleCache {
	#[instrument(skip_all, fields(id))]
	async fn get_parser(&self, _: ModuleId) -> Result<Arc<ThreadSafeParser>> {
		bail!(UNIMPLEMENTED)
	}

	async fn invalidate(&self, id: ModuleId) {
		// nothing is ever cached here yet
		self.parsers.remove(&id);
	}
}

#[async_trait]
impl IModuleDepProvider for LiveModuleCache {
	async fn get_module_deps(
		&self,
		_id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		bail!(UNIMPLEMENTED)
	}
}

#[async_trait]
impl IModuleCache for LiveModuleCache {
	async fn get_module_filepath(&self, _: ModuleId) -> Option<Url> {
		warn!("{UNIMPLEMENTED}");
		None
	}
	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		_id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		bail!(UNIMPLEMENTED)
	}
}
