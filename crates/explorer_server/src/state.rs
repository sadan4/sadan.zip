use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
	time::SystemTime,
};

use anyhow::{Context as _, Result};
use derive_more::Deref;
use explorer_server_core::{
	METADATA_FILE_NAME,
	build_has_full_data,
	get_build_path,
	get_root_build_path,
	read_mpk_zst_file,
};
use explorer_types::{BundleMetadata, Channel};
use tokio::{
	fs,
	sync::{RwLock, Semaphore},
	task::JoinSet,
};
use tracing::{info, instrument, warn};

use crate::cache::{Cache, RecentBlobs};

#[derive(Debug, Clone, Deref)]
pub struct State {
	#[deref]
	inner: Arc<RwLock<StateInner>>,
	pub cache: Arc<Cache>,
	/// bounds how many delta builds are rebuilt at once, each one holds
	/// ~500mb while it's rebuilt
	pub rebuild_permits: Arc<Semaphore>,
	/// rebuilt data of the newest delta builds served by `/full`, which works
	/// without redis and merges concurrent requests for one build
	pub rebuilt_full_data: Arc<RecentBlobs>,
}

/// see [`State::rebuild_permits`]
const MAX_CONCURRENT_REBUILDS: usize = 2;

/// see [`State::rebuilt_full_data`]. Each one is ~39mb
const RECENT_REBUILT_FULL_DATA: usize = 3;

#[derive(Debug, Default)]
pub struct StateInner {
	pub meta_by_time: BTreeMap<SystemTime, Arc<BundleMetadata>>,
	pub meta_by_hash: HashMap<String, Arc<BundleMetadata>>,
}

#[instrument(skip_all)]
async fn read_meta_entry(
	entry: fs::DirEntry,
) -> Result<Option<Arc<BundleMetadata>>> {
	let ft = entry.file_type().await?;
	if !ft.is_dir() {
		return Ok(None);
	}
	let entry_path = entry.path();
	let meta_path = entry_path.join(METADATA_FILE_NAME);
	if fs::read_dir(&entry_path)
		.await?
		.next_entry()
		.await?
		.is_none()
	{
		warn!("skipping empty build directory: {}", entry_path.display());
		return Ok(None);
	}
	let meta = tokio::task::spawn_blocking(move || -> Result<_> {
		let meta: BundleMetadata = read_mpk_zst_file(&meta_path)?;
		Ok(Some(Arc::new(meta)))
	})
	.await
	.context("Failed to join des thread")??;
	Ok(meta)
}

impl State {
	pub fn new(cache: Cache) -> Self {
		Self {
			inner: Arc::default(),
			cache: Arc::new(cache),
			rebuild_permits: Arc::new(Semaphore::new(MAX_CONCURRENT_REBUILDS)),
			rebuilt_full_data: Arc::new(RecentBlobs::new(
				RECENT_REBUILT_FULL_DATA,
			)),
		}
	}

	/// The newest stable build stored in full, which new canary builds are
	/// stored as deltas against
	pub async fn latest_full_stable(&self) -> Option<String> {
		let this = self.read().await;
		this.meta_by_time
			.values()
			.rev()
			.filter(|m| m.channel.contains(&Channel::Stable))
			.find(|m| {
				get_build_path(&m.build_hash)
					.is_ok_and(|p| build_has_full_data(&p))
			})
			.map(|m| m.build_hash.clone())
	}

	/// Whether `build_hash` is one of the `n` newest builds
	pub async fn is_recent_build(&self, build_hash: &str, n: usize) -> bool {
		self.read()
			.await
			.meta_by_time
			.values()
			.rev()
			.take(n)
			.any(|m| m.build_hash == build_hash)
	}

	pub async fn populate_from_disk(&self) -> Result<()> {
		info!("populating state from disk");
		let root_path =
			get_root_build_path().context("Failed to get root build path")?;
		let mut dirs = fs::read_dir(&root_path)
			.await
			.context("Failed to read root build directory")?;
		let cur_num_entries = self.read().await.meta_by_hash.len();
		let mut meta_by_hash = HashMap::with_capacity(cur_num_entries);
		let mut meta_by_time = BTreeMap::new();
		let mut js = JoinSet::new();
		while let Some(d) = dirs
			.next_entry()
			.await
			.context("Failed to get next dir entry")?
		{
			js.spawn(read_meta_entry(d));
		}
		while let Some(n) = js.join_next().await {
			let Some(n) = n
				.context("Join Error")?
				.context("Failed to read meta entry")?
			else {
				continue;
			};
			meta_by_hash.insert(n.build_hash.clone(), n.clone());
			meta_by_time.insert(n.first_seen_as_time(), n);
		}
		let mut this = self.write().await;
		this.meta_by_hash = meta_by_hash;
		this.meta_by_time = meta_by_time;
		drop(this);
		Ok(())
	}

	pub async fn add_build(&self, meta: BundleMetadata) {
		let meta = Arc::new(meta);
		let mut this = self.write().await;
		this.meta_by_hash
			.insert(meta.build_hash.clone(), meta.clone());
		this.meta_by_time
			.insert(meta.first_seen_as_time(), meta.clone());
	}
}
