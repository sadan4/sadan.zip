use std::{
	collections::HashMap,
	convert::Infallible,
	hash::Hasher,
	mem,
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use derive_more::{Deref, From};
use explorer_server_core::Channel;
use miette::{Context as _, Report, Result, bail};
use notify::{RecommendedWatcher, Watcher as _};
use rustc_hash::FxHasher;
use tokio::{fs, sync::mpsc};
use tracing::{debug, info, trace, warn};

use crate::{
	cmds::run::{FullReporterResult, run_reporter, run_with_data},
	fetcher::ScrapedOutput,
	util::MultiProgressWrapper,
	vc::{self, collect_plugins_from_paths},
};

type NotifyEvent = notify::Result<notify::Event>;

struct FsWatcher {
	inner: RecommendedWatcher,
	rx: mpsc::Receiver<NotifyEvent>,
}

impl FsWatcher {
	fn new() -> Result<Self> {
		let (tx, rx) = mpsc::channel(256);
		let watcher =
			notify::recommended_watcher(WatcherTx(tx)).map_err(Report::msg)?;
		Ok(Self { inner: watcher, rx })
	}
	fn watch_file(&mut self, path: &Path) -> Result<()> {
		self.inner
			.watch(path, notify::RecursiveMode::NonRecursive)
			.map_err(Report::msg)?;
		Ok(())
	}
}

#[derive(From, Deref)]
struct WatcherTx(#[deref] mpsc::Sender<NotifyEvent>);

impl notify::EventHandler for WatcherTx {
	fn handle_event(&mut self, event: NotifyEvent) {
		if let Err(e) = self.blocking_send(event) {
			warn!("Failed to send watcher event: {e:?}");
		}
	}
}

fn hash_contents(data: &[u8]) -> u64 {
	let mut hasher = FxHasher::default();
	hasher.write(data);
	hasher.finish()
}

/// Read a watched file, tolerating the window during which an editor has
/// truncated the file but not yet written its new contents.
///
/// Editors commonly save by opening the file with `O_TRUNC` and then writing
/// the new contents in a separate syscall. `notify` delivers the modify event
/// as soon as the truncate happens, so a naive read can observe the file while
/// it is momentarily empty (between the truncate and the write), yielding an
/// empty string. Rather than guessing a fixed sleep, retry briefly until the
/// file has contents.
async fn read_when_ready(path: &Path) -> Result<String> {
	const RETRY_DELAY: Duration = Duration::from_millis(1);
	/// Give up after this long so a genuinely-empty file can't hang the loop.
	const MAX_WAIT: Duration = Duration::from_millis(100);

	let mut waited = Duration::ZERO;
	loop {
		let contents = fs::read_to_string(&path)
			.await
			.map_err(Report::msg)?;
		if !contents.is_empty() {
			return Ok(contents);
		}
		if waited >= MAX_WAIT {
			bail!(
				"File {} is still empty after {}ms",
				path.display(),
				MAX_WAIT.as_millis()
			);
		}
		trace!(?path, "read empty file mid-write, retrying");
		tokio::time::sleep(RETRY_DELAY).await;
		waited += RETRY_DELAY;
	}
}

// TODO: exit success on ctrl-c
pub async fn run_watcher(
	cli: crate::Cli,
	global_bar: &MultiProgressWrapper,
) -> Result<Infallible> {
	info!("Starting watcher");
	let FullReporterResult {
		plugins, modules, ..
	} = run_reporter(&cli, global_bar)
		.await
		.map_err(Report::msg)?;
	global_bar.clear();
	info!("Finished initial run. Watching for file changes...");
	let mut watcher = FsWatcher::new()?;
	for crate::vc::Plugin { entry_point, .. } in &*plugins {
		watcher.watch_file(entry_point)?;
	}
	let mut changed_paths = Vec::new();
	// Map<PathBuf, hash(read(path))> to avoid running multiple times for the same change since editors often do noop writes, updating the modified time,
	let mut last_hash = HashMap::new();
	for plugin in &*plugins {
		let path = plugin.entry_point.clone();
		let hash = hash_contents(plugin.entry_source.as_bytes());
		last_hash.insert(path, hash);
	}
	while let Some(event) = watcher.rx.recv().await {
		fn match_event(event: NotifyEvent) -> Result<Option<Vec<PathBuf>>> {
			let mut event = match event
				.map_err(Report::msg)
				.context("Watcher Event")
			{
				Ok(event) => event,
				Err(e) => {
					warn!("{e:?}");
					return Ok(None);
				}
			};
			if !event.kind.is_modify() {
				trace!(?event, "Ignoring non-modify event");
				return Ok(None);
			}
			for path in &mut event.paths {
				*path = path
					.canonicalize()
					.map_err(Report::msg)?;
			}
			Ok(Some(event.paths))
		}
		if let Some(paths) = match_event(event)? {
			changed_paths.extend(paths);
		}
		loop {
			let paths = match watcher.rx.try_recv() {
				Ok(event) => match_event(event)?,
				Err(mpsc::error::TryRecvError::Disconnected) => {
					bail!("Watcher channel disconnected")
				}
				Err(mpsc::error::TryRecvError::Empty) => break,
			};
			let Some(paths) = paths else {
				continue;
			};
			changed_paths.extend(paths);
		}
		if changed_paths.is_empty() {
			continue;
		}
		debug!("Files changed. Re-running reporter...");
		let mut changed_contents = Vec::with_capacity(changed_paths.len());
		for path in mem::take(&mut changed_paths) {
			let contents = read_when_ready(&path).await?;
			let new_hash = hash_contents(contents.as_bytes());
			match last_hash.get_mut(&path) {
				Some(hash) if *hash == new_hash => {
					debug!(?path, "File contents unchanged, skipping");
					continue;
				}
				Some(hash) => {
					*hash = new_hash;
				}
				None => {
					last_hash.insert(path.clone(), new_hash);
				}
			}
			changed_contents.push((path, contents));
		}
		if changed_contents.is_empty() {
			debug!("Not re-running, no file contents changed");
			continue;
		}
		global_bar.clear();
		if !cli.is_trace() {
			global_bar.suspend(|| clearscreen::clear().unwrap());
		}
		info!("Plugins changed. Re-running reporter...");
		let new_plugins = collect_plugins_from_paths(changed_contents).await?;
		let _ =
			run_for_all_plugins(&cli, new_plugins, modules.clone(), global_bar)
				.await?;
		info!("Finished re-run");
	}
	bail!("File watcher channel closed.");
}

async fn run_for_all_plugins(
	cli: &crate::Cli,
	new_plugins: Vec<vc::Plugin>,
	builds: Vec<(Channel, Arc<ScrapedOutput>)>,
	global_bar: &MultiProgressWrapper,
) -> Result<FullReporterResult> {
	run_with_data(builds, Arc::new(new_plugins), cli, global_bar)
		.await
		.map_err(Report::msg)
}
