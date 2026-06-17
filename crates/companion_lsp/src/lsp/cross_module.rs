//! Cross-module parsing context for references / cross-file navigation.
//!
//! The expensive part of cross-module work is the eager pass over
//! `.modules/` to load every source and compute the inverse dep graph.
//! We split that into a shared, immutable `CrossModuleData` (Send+Sync,
//! cacheable across LSP requests) and a per-request `CrossModuleCtx`
//! which holds the ephemeral allocator + lazily-built parser cache and
//! borrows from `CrossModuleData` via an `Arc`.
//!
//! `WebpackAstParser::generate_references` needs `'ast`-bound lifetimes,
//! so `CrossModuleCtx` is still a self-referential pinned container —
//! it hands `&'static dyn IModuleCache<'static>` /
//! `&'static dyn IModuleDepProvider` references out via a self-pointer.
//! The `'static` is a lifetime fiction bounded by the lifetime of the
//! `CrossModuleCtx`; callers must not let parsers/references outlive
//! the ctx that produced them.

use std::{
	cell::RefCell,
	collections::HashMap,
	fs,
	marker::PhantomPinned,
	mem,
	path::{Path, PathBuf},
	pin::Pin,
	ptr,
	rc::Rc,
	sync::Arc,
};

use anyhow::{Context, Result};
use explorer_types::{IncomingModuleDeps, ModuleId};
use oxc::allocator::Allocator;
use smol_str::SmolStr;
use tower_lsp::lsp_types::Url;
use webpack_ast_parser::{
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

/// Shared, immutable precomputed view of `.modules/`. Send+Sync so we
/// can stash it on `SessionState` and reuse across requests.
///
/// Rebuilt only when the on-disk cache changes (download / purge).
pub struct CrossModuleData {
	root: PathBuf,
	/// Module sources, keyed by id. `String` is sufficient — the heap
	/// buffer has a stable address as long as the `String` isn't
	/// mutated or dropped, and we never do either after `build`.
	sources: HashMap<ModuleId, String>,
	/// `module_id` → modules that import it.
	inverse_deps: HashMap<ModuleId, IncomingModuleDeps>,
}

impl CrossModuleData {
	/// Eagerly walk `.modules/`, load every `<id>.js`, and parse each
	/// once to compute the inverse dep graph. The parsers used during
	/// the scan are dropped immediately — only sources + the graph stay.
	///
	/// Tries to load a precomputed `_cache.json` first; if present and the
	/// cached id set matches what's on disk, the (expensive) reparse pass
	/// is skipped entirely. Misses fall through to the full build and the
	/// fresh cache is written back to disk best-effort.
	pub fn build(root: PathBuf) -> Result<Self> {
		let sources = load_all_sources(&root)
			.with_context(|| format!("scanning {}", root.display()))?;

		if let Some(inverse_deps) = try_load_cache(&root, &sources) {
			tracing::debug!(
				modules = sources.len(),
				"cross-module data loaded from _cache.json"
			);
			return Ok(Self {
				root,
				sources,
				inverse_deps,
			});
		}

		let inverse_deps = build_inverse_deps(&sources);

		// Best-effort write — never fail the build if the disk cache can't
		// be persisted (read-only fs, races, etc.).
		if let Err(e) = write_cache(&root, &sources, &inverse_deps) {
			tracing::debug!(?e, "failed to write _cache.json");
		}

		Ok(Self {
			root,
			sources,
			inverse_deps,
		})
	}

	pub fn root(&self) -> &Path {
		&self.root
	}
}

const CACHE_FILE_NAME: &str = "_cache.json";
const CACHE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
	version: String,
	module_ids: Vec<u32>,
	inverse_deps: HashMap<u32, IncomingModuleDeps>,
}

fn try_load_cache(
	root: &Path,
	sources: &HashMap<ModuleId, String>,
) -> Option<HashMap<ModuleId, IncomingModuleDeps>> {
	let bytes = fs::read(root.join(CACHE_FILE_NAME)).ok()?;
	let cached: CacheFile = serde_json::from_slice(&bytes).ok()?;
	if cached.version != CACHE_VERSION {
		return None;
	}

	let mut on_disk: Vec<u32> = sources.keys().map(|id| id.0).collect();
	on_disk.sort_unstable();
	let mut cached_ids = cached.module_ids;
	cached_ids.sort_unstable();
	if cached_ids != on_disk {
		return None;
	}

	Some(
		cached
			.inverse_deps
			.into_iter()
			.map(|(id, deps)| (ModuleId(id), deps))
			.collect(),
	)
}

fn write_cache(
	root: &Path,
	sources: &HashMap<ModuleId, String>,
	inverse_deps: &HashMap<ModuleId, IncomingModuleDeps>,
) -> Result<()> {
	let mut module_ids: Vec<u32> = sources.keys().map(|id| id.0).collect();
	module_ids.sort_unstable();

	let payload = CacheFile {
		version: CACHE_VERSION.to_owned(),
		module_ids,
		inverse_deps: inverse_deps
			.iter()
			.map(|(id, deps)| (id.0, deps.clone()))
			.collect(),
	};

	let bytes = serde_json::to_vec(&payload)?;
	fs::write(root.join(CACHE_FILE_NAME), bytes)?;
	Ok(())
}

pub struct CrossModuleCtx {
	inner: Pin<Box<Inner>>,
}

struct Inner {
	/// Shared precomputed bits. Kept alive (via `Arc`) for the ctx's
	/// lifetime so the `&'static str`s into `data.sources` stay valid.
	data: Arc<CrossModuleData>,
	/// One allocator shared by every parser the ctx hands out. Boxed so
	/// it never moves once `Inner` is pinned.
	alloc: Box<Allocator>,
	/// Lazily-constructed parsers keyed by module id.
	parsers: RefCell<HashMap<ModuleId, Rc<WebpackAstParser<'static>>>>,
	/// Self-pointer; used to satisfy `&'ast dyn IModuleCache<'ast>`
	/// bounds without exposing the lifetime to callers.
	self_ptr: *const Self,
	_pin: PhantomPinned,
}

impl CrossModuleCtx {
	/// Construct a fresh per-request ctx from cached data. Cheap — just
	/// creates an allocator and an empty parser map.
	pub fn from_data(data: Arc<CrossModuleData>) -> Self {
		let inner = Inner {
			data,
			alloc: Box::new(Allocator::new()),
			parsers: RefCell::new(HashMap::new()),
			self_ptr: ptr::null(),
			_pin: PhantomPinned,
		};
		let mut pinned = Box::pin(inner);
		let self_ptr = &raw const *pinned;
		// SAFETY: `pinned` is `Pin<Box<Inner>>` — it never moves, so the
		// pointer remains valid for the lifetime of `Self`.
		unsafe {
			pinned
				.as_mut()
				.get_unchecked_mut()
				.self_ptr = self_ptr;
		};
		Self { inner: pinned }
	}

	/// Allocator handle for the "open" document parser. Lifetime is the
	/// `'static` fiction — the caller must keep the resulting parser
	/// inside the ctx's lifetime.
	pub fn alloc(&self) -> &'static Allocator {
		// SAFETY: `alloc` lives inside a `Pin<Box<Inner>>`; the address
		// is stable for the ctx's lifetime.
		unsafe {
			mem::transmute::<&Allocator, &'static Allocator>(&self.inner.alloc)
		}
	}

	pub fn cache_ref(&self) -> &'static dyn IModuleCache<'static> {
		// SAFETY: see `alloc`.
		unsafe { &*self.inner.self_ptr }
	}

	pub fn dep_provider_ref(&self) -> &'static dyn IModuleDepProvider {
		// SAFETY: see `alloc`.
		unsafe { &*self.inner.self_ptr }
	}

	/// Loaded source for a known module. Returns `None` if no `<id>.js`
	/// was present in the cache directory when the data was built.
	pub fn module_source(&self, id: ModuleId) -> Option<&str> {
		self.inner
			.data
			.sources
			.get(&id)
			.map(String::as_str)
	}

	pub fn module_file_uri(&self, id: ModuleId) -> Option<Url> {
		Url::from_file_path(
			self.inner
				.data
				.root
				.join(format!("{id}.js")),
		)
		.ok()
	}
}

impl Inner {
	fn get_or_make_parser(
		&self,
		id: ModuleId,
	) -> Result<Rc<WebpackAstParser<'static>>> {
		if let Some(p) = self.parsers.borrow().get(&id) {
			return Ok(p.clone());
		}
		let src = self
			.data
			.sources
			.get(&id)
			.with_context(|| format!("no cached source for module {id}"))?;
		// SAFETY: `data` is held by Arc for the ctx's lifetime, and the
		// `String`'s heap buffer is stable across that span (we never
		// mutate `data.sources` after `CrossModuleData::build`).
		let src_static: &'static str =
			unsafe { mem::transmute::<&str, &'static str>(src.as_str()) };
		// SAFETY: alloc is boxed inside Self, lives as long as Self.
		let alloc_static: &'static Allocator = unsafe {
			mem::transmute::<&Allocator, &'static Allocator>(&self.alloc)
		};
		let mut parser = WebpackAstParser::try_new(alloc_static, src_static)
			.with_context(|| format!("parse module {id}"))?;
		// SAFETY: self is pinned.
		let self_static: &'static Self = unsafe { &*self.self_ptr };
		parser.set_module_cache(self_static);
		parser.set_module_dep_provider(self_static);
		let parser = Rc::new(parser);
		self.parsers
			.borrow_mut()
			.insert(id, parser.clone());
		Ok(parser)
	}
}

impl IModuleCache<'static> for Inner {
	fn get_module_filepath(&self, id: ModuleId) -> Option<SmolStr> {
		let uri = Url::from_file_path(self.data.root.join(format!("{id}.js")))
			.ok()?;
		Some(uri.to_string().into())
	}

	fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'static>,
		id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Rc<WebpackAstParser<'static>>> {
		self.get_or_make_parser(id)
	}
}

impl IModuleDepProvider for Inner {
	fn get_module_deps(&self, id: ModuleId) -> Result<Rc<IncomingModuleDeps>> {
		// An absent entry means "no module imports this one" — surface
		// as an empty deps record rather than an error.
		Ok(Rc::new(
			self.data
				.inverse_deps
				.get(&id)
				.cloned()
				.unwrap_or_default(),
		))
	}
}

fn load_all_sources(root: &Path) -> Result<HashMap<ModuleId, String>> {
	let mut out = HashMap::new();
	for entry in fs::read_dir(root)? {
		let entry = entry?;
		let path = entry.path();
		if path
			.extension()
			.and_then(|e| e.to_str())
			!= Some("js")
		{
			continue;
		}
		let Some(stem) = path
			.file_stem()
			.and_then(|s| s.to_str())
		else {
			continue;
		};
		let Ok(id_n) = stem.parse::<u32>() else {
			continue;
		};
		let src = fs::read_to_string(&path)?;
		out.insert(ModuleId(id_n), src);
	}
	Ok(out)
}

fn build_inverse_deps(
	sources: &HashMap<ModuleId, String>,
) -> HashMap<ModuleId, IncomingModuleDeps> {
	let mut alloc = Allocator::new();
	let mut inv: HashMap<ModuleId, IncomingModuleDeps> = HashMap::new();
	for (id, src) in sources {
		let Ok(parser) = WebpackAstParser::try_new(&alloc, src.as_str()) else {
			continue;
		};
		let Some(outgoing) = parser.get_modules_that_this_module_requires()
		else {
			continue;
		};
		// Clone out before dropping the parser — `outgoing` borrows it.
		let sync = outgoing.sync.clone();
		let lazy = outgoing.lazy.clone();
		drop(parser);
		for dep in sync {
			inv.entry(dep)
				.or_default()
				.sync
				.push(*id);
		}
		for dep in lazy {
			inv.entry(dep)
				.or_default()
				.lazy
				.push(*id);
		}
		alloc.reset();
	}
	inv
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::TempDir;

	fn write_module(dir: &Path, id: u32, source: &str) {
		fs::write(dir.join(format!("{id}.js")), source).unwrap();
	}

	#[test]
	fn inverse_deps_inverts_outgoing_graph() {
		// Module 1 requires module 2; module 3 also requires module 2.
		// Expectation: 2's incoming = [1, 3].
		let tmp = TempDir::new().unwrap();
		write_module(
			tmp.path(),
			1,
			"// Webpack Module 1\n0,function(e,t,n){n(2)}\n",
		);
		write_module(
			tmp.path(),
			2,
			"// Webpack Module 2\n0,function(e,t,n){}\n",
		);
		write_module(
			tmp.path(),
			3,
			"// Webpack Module 3\n0,function(e,t,n){n(2)}\n",
		);

		let data = CrossModuleData::build(tmp.path().to_owned()).unwrap();
		let inc = data
			.inverse_deps
			.get(&ModuleId(2))
			.expect("module 2 should have incoming deps");
		let mut sync = inc.sync.clone();
		sync.sort();
		assert_eq!(sync, vec![ModuleId(1), ModuleId(3)]);
	}

	#[test]
	fn build_writes_cache_and_second_build_reads_it() {
		let tmp = TempDir::new().unwrap();
		write_module(
			tmp.path(),
			1,
			"// Webpack Module 1\n0,function(e,t,n){n(2)}\n",
		);
		write_module(
			tmp.path(),
			2,
			"// Webpack Module 2\n0,function(e,t,n){}\n",
		);

		let _ = CrossModuleData::build(tmp.path().to_owned()).unwrap();
		let cache_path = tmp.path().join(CACHE_FILE_NAME);
		assert!(cache_path.exists(), "_cache.json should be written");

		// Mutate the on-disk cache to a sentinel value; a second build that
		// reads the cache will surface the sentinel, proving the warm path
		// hit instead of re-running `build_inverse_deps`.
		let payload = CacheFile {
			version: CACHE_VERSION.to_owned(),
			module_ids: vec![1, 2],
			inverse_deps: HashMap::from([(
				2u32,
				IncomingModuleDeps {
					sync: vec![ModuleId(999)],
					lazy: vec![],
				},
			)]),
		};
		fs::write(&cache_path, serde_json::to_vec(&payload).unwrap()).unwrap();

		let data = CrossModuleData::build(tmp.path().to_owned()).unwrap();
		assert_eq!(
			data.inverse_deps
				.get(&ModuleId(2))
				.unwrap()
				.sync,
			vec![ModuleId(999)],
		);
	}

	#[test]
	fn build_ignores_cache_when_id_set_changes() {
		let tmp = TempDir::new().unwrap();
		write_module(
			tmp.path(),
			1,
			"// Webpack Module 1\n0,function(e,t,n){n(2)}\n",
		);
		write_module(
			tmp.path(),
			2,
			"// Webpack Module 2\n0,function(e,t,n){}\n",
		);

		// Cache file claims a module set that doesn't match disk.
		let payload = CacheFile {
			version: CACHE_VERSION.to_owned(),
			module_ids: vec![1, 2, 42],
			inverse_deps: HashMap::from([(
				2u32,
				IncomingModuleDeps {
					sync: vec![ModuleId(999)],
					lazy: vec![],
				},
			)]),
		};
		fs::write(
			tmp.path().join(CACHE_FILE_NAME),
			serde_json::to_vec(&payload).unwrap(),
		)
		.unwrap();

		let data = CrossModuleData::build(tmp.path().to_owned()).unwrap();
		// Should be the real computed value (1 requires 2), not the sentinel.
		assert_eq!(
			data.inverse_deps
				.get(&ModuleId(2))
				.unwrap()
				.sync,
			vec![ModuleId(1)],
		);
	}

	#[test]
	fn parser_creation_is_memoized() {
		let tmp = TempDir::new().unwrap();
		write_module(
			tmp.path(),
			7,
			"// Webpack Module 7\n0,function(e,t,n){}\n",
		);
		let data =
			Arc::new(CrossModuleData::build(tmp.path().to_owned()).unwrap());
		let ctx = CrossModuleCtx::from_data(data);
		let p1 = ctx
			.inner
			.get_or_make_parser(ModuleId(7))
			.unwrap();
		let p2 = ctx
			.inner
			.get_or_make_parser(ModuleId(7))
			.unwrap();
		assert!(Rc::ptr_eq(&p1, &p2));
	}
}
