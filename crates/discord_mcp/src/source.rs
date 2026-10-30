//! Loading builds from each supported source

use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{Context as _, Result, bail};
use bundle_source::{explorer::ExplorerClient, scrape};
use discord_scraper::ScrapeProgress;
use explorer_types::{
	Channel,
	DepInfo,
	FullBundle,
	IncomingModuleDeps,
	ModuleId,
};
use schemars::JsonSchema;
use serde::Deserialize;
use smol_str::{SmolStr, format_smolstr};
use tokio::task;
use tracing::{info, warn};
use webpack_ast_parser::WebpackAstParser;

use crate::store::{LoadedBundle, NewBundle, SourceKind};

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChannelArg {
	Stable,
	Canary,
}

impl From<ChannelArg> for Channel {
	fn from(value: ChannelArg) -> Self {
		match value {
			ChannelArg::Stable => Self::Stable,
			ChannelArg::Canary => Self::Canary,
		}
	}
}

/// Where to load a build from
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BuildSource {
	/// A build archived by the explorer server (<https://s-d-br.sadan.zip>).
	/// Fast, and comes with a dep graph, experiments and intl messages.
	Explorer {
		/// Build hash, from `list_builds`. Omit for the newest build the server
		/// has.
		#[serde(default)]
		hash: Option<String>,
	},
	/// Scrape the build currently live on discord.com. Slower, but always
	/// current. Raw modules are cached on disk by build hash.
	Scrape { channel: ChannelArg },
	/// A `.modules` directory written by the companion LSP's download modules
	/// command. These modules are pretty printed, so patches may not match
	/// them the way they match the real client.
	Local {
		/// Path to the directory holding `<id>.js` files
		path: String,
	},
}

pub async fn load(
	source: BuildSource,
	explorer: &ExplorerClient,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<LoadedBundle> {
	match source {
		BuildSource::Explorer { hash } => load_explorer(hash, explorer).await,
		BuildSource::Scrape { channel } => {
			load_scrape(channel.into(), progress).await
		}
		BuildSource::Local { path } => {
			task::spawn_blocking(move || load_local(Path::new(&path)))
				.await
				.context("Loading local modules panicked")?
		}
	}
}

async fn load_explorer(
	hash: Option<String>,
	explorer: &ExplorerClient,
) -> Result<LoadedBundle> {
	let hash = match hash {
		Some(hash) => hash.trim().to_owned(),
		None => {
			explorer
				.fetch_latest_build_meta()
				.await?
				.context("The explorer server has no builds")?
				.build_hash
		}
	};
	info!("Loading build {hash} from the explorer server");
	let FullBundle {
		metadata,
		dep_info,
		modules,
		experiments,
		css_chunks,
		..
	} = explorer
		.fetch_full_bundle(&hash)
		.await
		.with_context(|| format!("Failed to fetch build {hash}"))?;
	Ok(LoadedBundle::new(NewBundle {
		id: metadata.build_hash.into(),
		build_number: Some(metadata.build_number),
		channels: metadata.channel,
		source: SourceKind::Explorer,
		modules,
		preformatted: false,
		css_chunks,
		dep_info: Some(dep_info),
		experiments: Some(experiments),
		explorer: Some(explorer.clone()),
	}))
}

async fn load_scrape(
	channel: Channel,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<LoadedBundle> {
	let scrape::ScrapedBranch {
		channel,
		modules,
		build_hash,
		css_chunks,
	} = scrape::fetch_modules(channel, progress, true).await?;
	Ok(LoadedBundle::new(NewBundle {
		id: build_hash.into(),
		build_number: None,
		channels: vec![channel],
		source: SourceKind::Scrape(channel),
		modules,
		css_chunks,
		preformatted: false,
		dep_info: None,
		experiments: None,
		explorer: None,
	}))
}

/// Mirrors `CachedDepGraph` in `companion_lsp`
#[derive(Deserialize)]
struct CachedDepGraph {
	inverse_deps: HashMap<ModuleId, IncomingModuleDeps>,
}

const LSP_CACHE_FILE_NAME: &str = "_cache.json";

fn load_local(dir: &Path) -> Result<LoadedBundle> {
	let dir = dir
		.canonicalize()
		.with_context(|| format!("{} does not exist", dir.display()))?;
	if !dir.is_dir() {
		bail!("{} is not a directory", dir.display());
	}
	let mut modules = HashMap::new();
	for entry in std::fs::read_dir(&dir)? {
		let path = entry?.path();
		let Some(id) = module_id_of(&path) else {
			continue;
		};
		let src = std::fs::read_to_string(&path)
			.with_context(|| format!("Failed to read {}", path.display()))?;
		modules.insert(id, strip_module_header(src));
	}
	if modules.is_empty() {
		bail!("{} has no <module id>.js files", dir.display());
	}
	let dep_info = match std::fs::read(dir.join(LSP_CACHE_FILE_NAME)) {
		Ok(raw) => match serde_json::from_slice::<CachedDepGraph>(&raw) {
			Ok(graph) => Some(DepInfo {
				module_deps: graph.inverse_deps,
				..DepInfo::default()
			}),
			Err(e) => {
				warn!("Ignoring unreadable {LSP_CACHE_FILE_NAME}: {e}");
				None
			}
		},
		Err(_) => None,
	};
	let name = dir
		.file_name()
		.map_or_else(|| "root".into(), |n| n.to_string_lossy());
	let name = if name == ".modules" {
		dir.parent()
			.and_then(Path::file_name)
			.map_or(name, |n| n.to_string_lossy())
	} else {
		name
	};
	Ok(LoadedBundle::new(NewBundle {
		id: local_id(&name),
		build_number: None,
		channels: Vec::new(),
		source: SourceKind::Local(PathBuf::from(&dir)),
		modules,
		css_chunks: HashMap::new(),
		preformatted: true,
		dep_info,
		experiments: None,
		explorer: None,
	}))
}

fn local_id(name: &str) -> SmolStr {
	let name = name
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
				c
			} else {
				'_'
			}
		})
		.collect::<String>();
	format_smolstr!("local-{name}")
}

fn module_id_of(path: &Path) -> Option<ModuleId> {
	if path.extension()? != "js" {
		return None;
	}
	path.file_stem()?
		.to_str()?
		.parse()
		.ok()
		.map(ModuleId)
}

/// Removes the `// Webpack Module` header the LSP writes, up to and including
/// the `0,` line, so the module looks like a scraped one
fn strip_module_header(src: String) -> String {
	if !WebpackAstParser::is_webpack_module(&src) {
		return src;
	}
	let mut offset = 0;
	for line in src.split_inclusive('\n') {
		let trimmed = line.trim();
		offset += line.len();
		if trimmed == "0," {
			return src[offset..].to_owned();
		}
		if !trimmed.starts_with("//") {
			break;
		}
	}
	src
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn strips_lsp_header() {
		let src = "// Webpack Module 12\n//EXTRACTED WEBPACK MODULE 12\n0,\nfunction(e) {}\n";
		assert_eq!(strip_module_header(src.into()), "function(e) {}\n");
		let plain = "function(e) {}";
		assert_eq!(strip_module_header(plain.into()), plain);
	}

	#[test]
	fn loads_local_dir() {
		let dir = tempfile::tempdir().unwrap();
		let modules = dir.path().join(".modules");
		std::fs::create_dir_all(&modules).unwrap();
		std::fs::write(
			modules.join("1.js"),
			"// Webpack Module 1\n0,\nfunction(e,t,n){n(2)}",
		)
		.unwrap();
		std::fs::write(modules.join("2.js"), "function(e){}").unwrap();
		std::fs::write(modules.join("notes.txt"), "ignored").unwrap();
		std::fs::write(
			modules.join(LSP_CACHE_FILE_NAME),
			r#"{"version":"1","inverse_deps":{"2":{"sync":[1],"lazy":[]}}}"#,
		)
		.unwrap();
		let bundle = load_local(&modules).unwrap();
		assert_eq!(bundle.modules.len(), 2);
		assert_eq!(bundle.raw(ModuleId(1)).unwrap(), "function(e,t,n){n(2)}");
		assert!(bundle.id.starts_with("local-"));
	}
}
