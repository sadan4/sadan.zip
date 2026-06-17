//! On-disk cache of webpack modules extracted from Discord. Layout matches
//! the legacy plugin so users with an existing `.modules/` directory pick up
//! seamlessly: each module is stored at `<workspace>/.modules/<id>.js`,
//! pretty-printed via `pretty_printer::format_to_str` for readable
//! navigation.

use std::{
	collections::HashSet,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use tokio::fs;

/// Directory name (relative to the workspace root) used for the on-disk
/// cache. Matches the legacy plugin's hardcoded path.
const CACHE_DIR_NAME: &str = ".modules";

#[derive(Debug, Default)]
pub struct ModuleCache {
	/// Resolved cache directory once a workspace is known. `None` before
	/// `initialize` or when the editor opens with no workspace.
	root: Option<PathBuf>,
	/// Set of module IDs already on disk. Lazily populated when the cache
	/// directory is scanned.
	known: HashSet<String>,
}

impl ModuleCache {
	pub fn new() -> Self {
		Self::default()
	}

	/// Bind the cache to a workspace root (the parent of `.modules/`).
	/// Idempotent — re-binding to a new root clears the in-memory index.
	pub fn set_workspace_root(&mut self, workspace: &Path) {
		let root = workspace.join(CACHE_DIR_NAME);
		if self.root.as_deref() != Some(&root) {
			self.root = Some(root);
			self.known.clear();
		}
	}

	pub fn root(&self) -> Option<&Path> {
		self.root.as_deref()
	}

	/// True once `.modules/` exists on disk.
	pub async fn has_cache(&self) -> bool {
		match &self.root {
			Some(p) => fs::try_exists(p).await.unwrap_or(false),
			None => false,
		}
	}

	pub async fn ensure_root(&self) -> Result<&Path> {
		let root = self
			.root
			.as_deref()
			.context("module cache has no workspace yet")?;
		fs::create_dir_all(root)
			.await
			.with_context(|| format!("create_dir_all({})", root.display()))?;
		Ok(root)
	}

	pub async fn write_module(
		&mut self,
		id: &str,
		formatted: &str,
	) -> Result<PathBuf> {
		let root = self.ensure_root().await?.to_owned();
		let path = root.join(format!("{id}.js"));
		fs::write(&path, formatted)
			.await
			.with_context(|| format!("write {}", path.display()))?;
		self.known.insert(id.to_owned());
		Ok(path)
	}

	pub async fn read_module(&self, id: &str) -> Result<Option<String>> {
		let Some(root) = &self.root else {
			return Ok(None);
		};
		let path = root.join(format!("{id}.js"));
		match fs::read_to_string(&path).await {
			Ok(s) => Ok(Some(s)),
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
			Err(e) => {
				Err(e).with_context(|| format!("read {}", path.display()))
			}
		}
	}

	pub fn module_path(&self, id: &str) -> Option<PathBuf> {
		self.root
			.as_ref()
			.map(|r| r.join(format!("{id}.js")))
	}

	pub async fn purge(&mut self) -> Result<()> {
		let Some(root) = &self.root else {
			return Ok(());
		};
		if fs::try_exists(root)
			.await
			.unwrap_or(false)
		{
			fs::remove_dir_all(root)
				.await
				.with_context(|| {
					format!("remove_dir_all({})", root.display())
				})?;
		}
		self.known.clear();
		Ok(())
	}

	/// Walk `.modules/` and refresh the in-memory id index.
	pub async fn rescan(&mut self) -> Result<()> {
		self.known.clear();
		let Some(root) = self.root.clone() else {
			return Ok(());
		};
		if !fs::try_exists(&root)
			.await
			.unwrap_or(false)
		{
			return Ok(());
		}
		let mut entries = fs::read_dir(&root).await?;
		while let Some(entry) = entries.next_entry().await? {
			let name = entry.file_name();
			let name = name.to_string_lossy();
			if let Some(stem) = name.strip_suffix(".js") {
				self.known.insert(stem.to_owned());
			}
		}
		Ok(())
	}

	pub fn known_ids(&self) -> impl Iterator<Item = &str> {
		self.known.iter().map(String::as_str)
	}

	pub fn module_count(&self) -> usize {
		self.known.len()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::TempDir;

	fn tmp_cache() -> (TempDir, ModuleCache) {
		let dir = TempDir::new().unwrap();
		let mut cache = ModuleCache::new();
		cache.set_workspace_root(dir.path());
		(dir, cache)
	}

	#[tokio::test]
	async fn write_then_read_round_trips() {
		let (_dir, mut cache) = tmp_cache();
		cache
			.write_module("123", "function a(){}")
			.await
			.unwrap();
		assert_eq!(
			cache
				.read_module("123")
				.await
				.unwrap()
				.as_deref(),
			Some("function a(){}"),
		);
		assert_eq!(cache.module_count(), 1);
	}

	#[tokio::test]
	async fn read_missing_module_is_none() {
		let (_dir, cache) = tmp_cache();
		assert!(
			cache
				.read_module("999")
				.await
				.unwrap()
				.is_none()
		);
	}

	#[tokio::test]
	async fn purge_clears_disk_and_index() {
		let (_dir, mut cache) = tmp_cache();
		cache
			.write_module("1", "x")
			.await
			.unwrap();
		assert!(cache.has_cache().await);
		cache.purge().await.unwrap();
		assert!(!cache.has_cache().await);
		assert_eq!(cache.module_count(), 0);
	}

	#[tokio::test]
	async fn rescan_picks_up_existing_files() {
		let (_dir, mut cache) = tmp_cache();
		cache
			.write_module("42", "y")
			.await
			.unwrap();
		// Drop the in-memory state and re-scan.
		let known_before = cache.module_count();
		let mut fresh = ModuleCache::new();
		fresh.set_workspace_root(cache.root().unwrap().parent().unwrap());
		fresh.rescan().await.unwrap();
		assert_eq!(fresh.module_count(), known_before);
		assert!(fresh.known_ids().any(|i| i == "42"));
	}
}
