use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
	time::SystemTime,
};

use anyhow::{Context as _, Result};
use derive_more::Deref;
use explorer_server_core::{METADATA_FILE_NAME, get_root_build_path};
use explorer_types::BundleMetadata;
use tokio::{fs, sync::RwLock, task::JoinSet};
use tracing::info;

#[derive(Debug, Clone, Default, Deref)]
pub struct State(Arc<RwLock<StateInner>>);

#[derive(Debug, Default)]
pub struct StateInner {
	pub meta_by_time: BTreeMap<SystemTime, Arc<BundleMetadata>>,
	pub meta_by_hash: HashMap<String, Arc<BundleMetadata>>,
}

async fn read_meta_entry(
	entry: fs::DirEntry,
) -> Result<Option<Arc<BundleMetadata>>> {
	let ft = entry.file_type().await?;
	if !ft.is_dir() {
		return Ok(None);
	}
	let meta_path = entry.path().join(METADATA_FILE_NAME);
	let meta_zstd_raw = fs::read(&meta_path)
		.await
		.with_context(|| {
			format!(
				"Failed to read meta data for path: {}",
				meta_path.display()
			)
		})?;
	let meta = tokio::task::spawn_blocking(move || -> Result<_> {
		let meta_raw =
			zstd::decode_all(&*meta_zstd_raw).context("ZSTD Error")?;
		let meta = rmp_serde::from_slice(&meta_raw).context("RMP error")?;
		Ok(Some(Arc::new(meta)))
	})
	.await
	.context("Failed to join des thread")??;
	Ok(meta)
}

impl State {
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
