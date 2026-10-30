//! Loaded builds and everything lazily derived from them

use std::{
	collections::HashMap,
	fmt::Write as _,
	path::PathBuf,
	sync::{Arc, Mutex, RwLock},
};

use anyhow::{Context as _, Result, anyhow, bail};
use bundle_source::explorer::ExplorerClient;
use dashmap::DashMap;
use discord_scraper::{NoProgress, ScrapeProgress, experiments::ParsedBundle};
use explorer_types::{
	Channel,
	DepInfo,
	IncomingModuleDeps,
	ModuleId,
	Modules,
	experiments::Experiment,
	intl::IntlMessages,
};
use pretty_printer::FormattedContent;
use smol_str::SmolStr;
use tokio::{sync::OnceCell, task};
use webpack_ast_parser::{ThreadSafeParser, WebpackAstParser};

use crate::{css::CssIndex, text::LineIndex};

const INDENT: u8 = 4;

/// How many builds are kept in memory at once
pub const MAX_LOADED: usize = 2;

#[derive(Debug, Clone)]
pub enum SourceKind {
	Explorer,
	Scrape(Channel),
	Local(PathBuf),
}

/// A module pretty printed for display, with a way to map positions in the
/// unformatted module onto it
pub struct FormattedModule {
	pub code: String,
	/// `None` if the module could not be formatted, so [`Self::code`] is the
	/// unformatted module
	mappings: Option<Vec<(u32, u32)>>,
	pub lines: LineIndex,
}

impl FormattedModule {
	/// Maps a byte offset in the parsed module (with its header) to a byte
	/// offset in [`Self::code`]
	pub fn map_pos(&self, pos: u32) -> usize {
		match &self.mappings {
			Some(m) => pretty_printer::map_pos(m, pos) as usize,
			None => pos as usize,
		}
	}

	/// 1-based line in [`Self::code`] of a byte offset in the parsed module
	pub fn line_of(&self, pos: u32) -> usize {
		self.lines.line(self.map_pos(pos))
	}
}

pub struct LoadedBundle {
	/// the build hash, or `local-<dir name>` for a local module dir
	pub id: SmolStr,
	pub build_number: Option<u32>,
	pub channels: Vec<Channel>,
	pub source: SourceKind,
	/// raw module sources, without the `// Webpack Module` header
	pub modules: Arc<Modules>,
	/// css chunk file name to its minified contents
	css_chunks: HashMap<SmolStr, String>,
	/// the module sources are already pretty printed, so patches written
	/// against the minified client may not match them
	pub preformatted: bool,
	/// the dep graph that came with the build, used until [`Self::parsed`]
	/// takes it
	dep_info: Mutex<Option<DepInfo>>,
	explorer: Option<ExplorerClient>,
	parsed: OnceCell<Arc<ParsedBundle>>,
	intl: OnceCell<Arc<IntlMessages>>,
	experiments: OnceCell<Arc<Vec<Experiment>>>,
	formatted: DashMap<ModuleId, Arc<FormattedModule>>,
	/// lazily computed results that are expensive to find
	pub caches: Caches,
}

#[derive(Default)]
pub struct Caches {
	pub css: OnceCell<Arc<Vec<webpack_ast_parser::css::ClassName>>>,
	pub endpoints: OnceCell<Arc<crate::tools::misc::EndpointsModule>>,
	pub css_index: OnceCell<Arc<CssIndex>>,
}

pub struct NewBundle {
	pub id: SmolStr,
	pub build_number: Option<u32>,
	pub channels: Vec<Channel>,
	pub source: SourceKind,
	pub modules: Modules,
	pub css_chunks: HashMap<SmolStr, String>,
	pub preformatted: bool,
	pub dep_info: Option<DepInfo>,
	pub experiments: Option<Vec<Experiment>>,
	pub explorer: Option<ExplorerClient>,
}

impl LoadedBundle {
	pub fn new(b: NewBundle) -> Self {
		let experiments = OnceCell::new_with(b.experiments.map(Arc::new));
		Self {
			id: b.id,
			build_number: b.build_number,
			channels: b.channels,
			source: b.source,
			modules: Arc::new(b.modules),
			css_chunks: b.css_chunks,
			preformatted: b.preformatted,
			dep_info: Mutex::new(b.dep_info),
			explorer: b.explorer,
			parsed: OnceCell::new(),
			intl: OnceCell::new(),
			experiments,
			formatted: DashMap::new(),
			caches: Caches::default(),
		}
	}

	pub fn raw(&self, id: ModuleId) -> Result<&str> {
		self.modules
			.get(&id)
			.map(String::as_str)
			.ok_or_else(|| {
				anyhow!("Module {id} not found in build {}", self.id)
			})
	}

	/// The module source as it is parsed, with the `// Webpack Module` header
	pub fn with_header(&self, id: ModuleId) -> Result<String> {
		let mut src = self.raw(id)?.to_owned();
		WebpackAstParser::format_module_header(&mut src, id, false);
		Ok(src)
	}

	pub fn sorted_ids(&self) -> Vec<ModuleId> {
		let mut ids = self
			.modules
			.keys()
			.copied()
			.collect::<Vec<_>>();
		ids.sort_unstable();
		ids
	}

	pub fn formatted(&self, id: ModuleId) -> Result<Arc<FormattedModule>> {
		if let Some(f) = self.formatted.get(&id) {
			return Ok(f.clone());
		}
		let src = self.with_header(id)?;
		let fmt = match pretty_printer::format(&src, INDENT) {
			Ok(FormattedContent { code, mappings }) => FormattedModule {
				lines: LineIndex::new(&code),
				code,
				mappings: Some(mappings),
			},
			Err(_) => FormattedModule {
				lines: LineIndex::new(&src),
				code: src,
				mappings: None,
			},
		};
		let fmt = Arc::new(fmt);
		self.formatted.insert(id, fmt.clone());
		Ok(fmt)
	}

	/// Every module parsed, with cross module lookups wired up
	///
	/// The first call parses the whole build, which takes a while
	pub async fn parsed(
		self: &Arc<Self>,
		progress: Arc<dyn ScrapeProgress>,
	) -> Result<Arc<ParsedBundle>> {
		self.parsed
			.get_or_try_init(|| {
				let this = self.clone();
				async move {
					task::spawn_blocking(move || {
						let dep_info = this.dep_info.lock().unwrap().take();
						let parsed = match dep_info {
							Some(dep_info) => ParsedBundle::with_dep_info(
								&this.modules,
								dep_info,
								progress,
							),
							None => ParsedBundle::new(&this.modules, progress),
						}?;
						anyhow::Ok(Arc::new(parsed))
					})
					.await
					.context("Parsing the build panicked")?
				}
			})
			.await
			.cloned()
	}

	/// Every css chunk, formatted and indexed
	pub async fn css(self: &Arc<Self>) -> Result<Arc<CssIndex>> {
		if self.css_chunks.is_empty() {
			bail!(
				"Build {} has no css. Builds loaded from a local module dir \
				 don't include it; load one from the explorer or a scrape.",
				self.id
			);
		}
		self.caches
			.css_index
			.get_or_try_init(|| {
				let this = self.clone();
				async move {
					task::spawn_blocking(move || {
						Arc::new(CssIndex::new(&this.css_chunks))
					})
					.await
					.context("Indexing css panicked")
				}
			})
			.await
			.cloned()
	}

	pub fn is_parsed(&self) -> bool {
		self.parsed.initialized()
	}

	/// A parser for just this module, without parsing the whole build
	///
	/// Cross module lookups will fail on it unless the build is already
	/// parsed
	pub fn standalone_parser(
		&self,
		id: ModuleId,
	) -> Result<Arc<ThreadSafeParser>> {
		if let Some(parsed) = self.parsed.get()
			&& let Some(parser) = parsed.get_parser(id)
		{
			return Ok(parser);
		}
		let src = self.with_header(id)?;
		let parser =
			ThreadSafeParser::new(src.clone().into()).map_err(|e| {
				anyhow!(pretty_printer::render_diag(
					e,
					&src,
					&format!("{id}.js")
				))
			})?;
		Ok(Arc::new(parser))
	}

	/// The modules that require `id`, without parsing the whole build if the
	/// build came with a dep graph
	pub async fn incoming_deps(
		self: &Arc<Self>,
		id: ModuleId,
	) -> Result<IncomingModuleDeps> {
		if !self.is_parsed()
			&& let Some(dep_info) = &*self.dep_info.lock().unwrap()
		{
			return Ok(dep_info
				.module_deps
				.get(&id)
				.cloned()
				.unwrap_or_default());
		}
		let parsed = self
			.parsed(Arc::new(NoProgress))
			.await?;
		Ok(parsed
			.dep_info()
			.module_deps
			.get(&id)
			.cloned()
			.unwrap_or_default())
	}

	pub async fn intl(
		self: &Arc<Self>,
		progress: Arc<dyn ScrapeProgress>,
	) -> Result<Arc<IntlMessages>> {
		self.intl
			.get_or_try_init(|| async {
				if let Some(explorer) = &self.explorer
					&& let Some(intl) = explorer.fetch_intl(&self.id).await?
				{
					return Ok(Arc::new(intl));
				}
				let parsed = self.parsed(progress).await?;
				Ok(Arc::new(parsed.find_intl().await?))
			})
			.await
			.cloned()
	}

	pub async fn experiments(
		self: &Arc<Self>,
		progress: Arc<dyn ScrapeProgress>,
	) -> Result<Arc<Vec<Experiment>>> {
		self.experiments
			.get_or_try_init(|| async {
				let parsed = self.parsed(progress).await?;
				Ok(Arc::new(parsed.find_experiments().await?))
			})
			.await
			.cloned()
	}

	pub fn summary(&self) -> String {
		let mut out = format!("build {}", self.id);
		if let Some(n) = self.build_number {
			write!(out, " (#{n})").unwrap();
		}
		if !self.channels.is_empty() {
			write!(out, ", channels {:?}", self.channels).unwrap();
		}
		match &self.source {
			SourceKind::Explorer => out.push_str(", from the explorer server"),
			SourceKind::Scrape(ch) => {
				write!(out, ", scraped from {ch:?}").unwrap();
			}
			SourceKind::Local(path) => {
				write!(out, ", from {}", path.display()).unwrap();
			}
		}
		write!(
			out,
			", {} modules, {} css chunks",
			self.modules.len(),
			self.css_chunks.len()
		)
		.unwrap();
		if self.is_parsed() {
			out.push_str(", parsed");
		}
		out
	}
}

/// Every loaded build, most recently loaded first
pub struct BundleStore {
	bundles: RwLock<Vec<Arc<LoadedBundle>>>,
	max: usize,
}

impl Default for BundleStore {
	fn default() -> Self {
		Self::new(MAX_LOADED)
	}
}

impl BundleStore {
	pub fn new(max: usize) -> Self {
		Self {
			bundles: RwLock::new(Vec::new()),
			max: max.max(1),
		}
	}

	/// Adds `bundle` as the active build, evicting the least recently loaded
	/// build if there are too many
	///
	/// Returns the ids of evicted builds
	pub fn insert(&self, bundle: Arc<LoadedBundle>) -> Vec<SmolStr> {
		let mut bundles = self.bundles.write().unwrap();
		bundles.retain(|b| b.id != bundle.id);
		bundles.insert(0, bundle);
		let keep = self.max.min(bundles.len());
		bundles
			.drain(keep..)
			.map(|b| b.id.clone())
			.collect()
	}

	pub fn all(&self) -> Vec<Arc<LoadedBundle>> {
		self.bundles.read().unwrap().clone()
	}

	/// Finds a loaded build by id, unique id prefix, or channel name
	///
	/// `None` is the active build
	pub fn get(&self, sel: Option<&str>) -> Result<Arc<LoadedBundle>> {
		let bundles = self.bundles.read().unwrap();
		let Some(sel) = sel
			.map(str::trim)
			.filter(|s| !s.is_empty())
		else {
			return bundles.first().cloned().context(
				"No build is loaded. Call load_build first, for example with \
				 {\"source\": {\"kind\": \"explorer\"}} for the latest build",
			);
		};
		if let Some(b) = bundles.iter().find(|b| b.id == sel) {
			return Ok(b.clone());
		}
		let channel = match sel.to_ascii_lowercase().as_str() {
			"stable" => Some(Channel::Stable),
			"canary" => Some(Channel::Canary),
			_ => None,
		};
		let matches = bundles
			.iter()
			.filter(|b| match channel {
				Some(ch) => b.channels.contains(&ch),
				None => b.id.starts_with(sel),
			})
			.collect::<Vec<_>>();
		match matches.as_slice() {
			[b] => Ok((*b).clone()),
			[] => bail!(
				"No loaded build matches {sel:?}. Loaded builds: {}",
				ids(&bundles)
			),
			_ => bail!(
				"{sel:?} matches more than one loaded build: {}",
				matches
					.iter()
					.map(|b| b.id.as_str())
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}
}

fn ids(bundles: &[Arc<LoadedBundle>]) -> String {
	if bundles.is_empty() {
		return String::from("(none)");
	}
	bundles
		.iter()
		.map(|b| b.id.as_str())
		.collect::<Vec<_>>()
		.join(", ")
}

/// Parses a module id the way agents tend to write them
pub fn parse_module_id(s: &str) -> Result<ModuleId> {
	s.trim()
		.trim_end_matches(".js")
		.parse::<u32>()
		.map(ModuleId)
		.with_context(|| format!("{s:?} is not a module id"))
}

#[cfg(test)]
pub mod test_util {
	use super::*;

	pub fn bundle(id: &str, modules: &[(u32, &str)]) -> Arc<LoadedBundle> {
		bundle_with_css(id, modules, &[])
	}

	pub fn bundle_with_css(
		id: &str,
		modules: &[(u32, &str)],
		css: &[(&str, &str)],
	) -> Arc<LoadedBundle> {
		Arc::new(LoadedBundle::new(NewBundle {
			id: id.into(),
			build_number: Some(1),
			channels: vec![Channel::Stable],
			source: SourceKind::Explorer,
			modules: modules
				.iter()
				.map(|&(id, src)| (ModuleId(id), src.to_owned()))
				.collect::<HashMap<_, _>>(),
			css_chunks: css
				.iter()
				.map(|&(name, src)| (name.into(), src.to_owned()))
				.collect(),
			preformatted: false,
			dep_info: None,
			experiments: Some(Vec::new()),
			explorer: None,
		}))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn store_selects_and_evicts() {
		let store = BundleStore::new(2);
		store.insert(test_util::bundle("aaa111", &[]));
		store.insert(test_util::bundle("bbb222", &[]));
		assert_eq!(store.get(None).unwrap().id, "bbb222");
		assert_eq!(store.get(Some("aaa")).unwrap().id, "aaa111");
		let evicted = store.insert(test_util::bundle("ccc333", &[]));
		assert_eq!(evicted, ["aaa111"]);
		assert!(store.get(Some("aaa")).is_err());
		assert!(store.get(Some("stable")).is_err(), "two stable builds");
	}

	#[test]
	fn parses_module_ids() {
		assert_eq!(parse_module_id(" 123.js").unwrap(), ModuleId(123));
		assert!(parse_module_id("abc").is_err());
	}
}
