use std::{collections::VecDeque, sync::Arc};

use anyhow::{Context, Result};
use axum::body::Bytes;
use redis::AsyncCommands as _;
use tokio::sync::OnceCell;

/// The few most recently requested blobs of one kind, kept in memory.
///
/// Concurrent requests for the same build share one [`OnceCell`], so a blob
/// is only built once no matter how many requests are waiting on it.
#[derive(Debug)]
pub struct RecentBlobs {
	capacity: usize,
	/// most recently used first
	entries: std::sync::Mutex<VecDeque<(String, Arc<OnceCell<Bytes>>)>>,
}

impl RecentBlobs {
	pub fn new(capacity: usize) -> Self {
		Self {
			capacity,
			entries: std::sync::Mutex::new(VecDeque::with_capacity(capacity)),
		}
	}

	/// Returns the cell for `build_hash`, inserting an empty one if it isn't
	/// tracked, and marks it as the most recently used.
	///
	/// A cell that failed to initialize stays empty, so the next caller
	/// retries.
	pub fn cell(&self, build_hash: &str) -> Arc<OnceCell<Bytes>> {
		let mut entries = self
			.entries
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);
		let entry = match entries
			.iter()
			.position(|(h, _)| h == build_hash)
		{
			Some(i) => entries
				.remove(i)
				.expect("index is in bounds"),
			None => (build_hash.to_owned(), Arc::default()),
		};
		let cell = entry.1.clone();
		entries.push_front(entry);
		// in-flight requests keep their own `Arc`, so evicting never cancels
		// them
		entries.truncate(self.capacity);
		cell
	}
}

/// What a cached blob is, which decides its key and how long it lives
#[derive(Debug, Clone, Copy)]
pub enum CacheKind {
	/// a `.7z` archive of a build
	Archive,
	/// the data file of a build stored as a delta, rebuilt and recompressed
	FullData,
}

impl CacheKind {
	const fn key_prefix(self) -> &'static str {
		match self {
			Self::Archive => "discord-build-archive:v3:",
			Self::FullData => "discord-build-full:v1:",
		}
	}

	const fn ttl(self) -> u64 {
		match self {
			// 7 days
			Self::Archive => 60 * 60 * 24 * 7,
			// 1 day, canary builds are mostly looked at while they're new.
			// only the newest builds are cached at all, see
			// `REDIS_FULL_DATA_RECENT_BUILDS`
			Self::FullData => 60 * 60 * 24,
		}
	}
}

#[derive(Debug, Default, Clone)]
pub struct Cache {
	redis: Option<redis::aio::MultiplexedConnection>,
}

impl Cache {
	pub const fn new() -> Self {
		Self { redis: None }
	}

	/// Returns [`None`] when no cache is configured, or when nothing is
	/// cached.
	pub async fn get(
		&self,
		kind: CacheKind,
		build_hash: &str,
	) -> Result<Option<Vec<u8>>> {
		let Some(conn) = &self.redis else {
			return Ok(None);
		};
		let mut conn = conn.clone();
		let blob = conn
			.get::<_, Option<Vec<u8>>>(format!(
				"{}{build_hash}",
				kind.key_prefix()
			))
			.await
			.with_context(|| {
				format!("Failed to get cached {kind:?} from redis")
			})?;
		Ok(blob)
	}

	/// Does nothing when no cache is configured.
	pub async fn set(
		&self,
		kind: CacheKind,
		build_hash: &str,
		blob: &[u8],
	) -> Result<()> {
		let Some(conn) = &self.redis else {
			return Ok(());
		};
		// cheap, every clone shares the one connection
		let mut conn = conn.clone();
		conn.set_ex::<_, _, ()>(
			format!("{}{build_hash}", kind.key_prefix()),
			blob,
			kind.ttl(),
		)
		.await
		.with_context(|| format!("Failed to cache {kind:?} in redis"))?;
		Ok(())
	}

	pub async fn connect(uri: &str) -> Result<Self> {
		let redis = redis::Client::open(uri)
			.context("Failed to create redis client")?;
		let mut conn = redis
			.get_multiplexed_async_connection()
			.await
			.context("Failed to connect to redis")?;
		// opening a client doesn't actually connect, we need to ping to verify the connection
		redis::cmd("PING")
			.exec_async(&mut conn)
			.await
			.context("Redis PING failed")?;
		Ok(Self { redis: Some(conn) })
	}
}

#[cfg(test)]
mod tests {
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};

	use axum::body::Bytes;

	use super::RecentBlobs;

	#[test]
	fn evicts_least_recently_used() {
		let blobs = RecentBlobs::new(2);
		let a = blobs.cell("a");
		let b = blobs.cell("b");
		// touching `a` makes `b` the oldest
		assert!(Arc::ptr_eq(&a, &blobs.cell("a")));
		blobs.cell("c");
		assert!(Arc::ptr_eq(&a, &blobs.cell("a")));
		assert!(!Arc::ptr_eq(&b, &blobs.cell("b")));
	}

	#[tokio::test]
	async fn concurrent_requests_build_once() {
		let blobs = RecentBlobs::new(2);
		let builds = AtomicUsize::new(0);
		let get = || async {
			blobs
				.cell("a")
				.get_or_init(|| async {
					builds.fetch_add(1, Ordering::SeqCst);
					tokio::task::yield_now().await;
					Bytes::from_static(b"data")
				})
				.await
				.clone()
		};
		let (x, y) = tokio::join!(get(), get());
		assert_eq!(x, y);
		assert_eq!(builds.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn failed_build_is_retried() {
		let blobs = RecentBlobs::new(2);
		let failed = blobs
			.cell("a")
			.get_or_try_init(|| async { Err(()) })
			.await
			.cloned();
		assert!(failed.is_err());
		let ok = blobs
			.cell("a")
			.get_or_try_init(|| async {
				Ok::<_, ()>(Bytes::from_static(b"data"))
			})
			.await
			.cloned();
		assert_eq!(ok, Ok(Bytes::from_static(b"data")));
	}
}
