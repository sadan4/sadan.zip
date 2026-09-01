use anyhow::{Context, Result};
use redis::AsyncCommands as _;

const ARCHIVE_KEY_PREFIX: &str = "discord-build-archive:";
/// 7 days
const ARCHIVE_TTL: u64 = 60 * 60 * 24 * 7;

#[derive(Debug, Default, Clone)]
pub struct Cache {
	redis: Option<redis::aio::MultiplexedConnection>,
}

impl Cache {
	pub const fn new() -> Self {
		Self { redis: None }
	}

	/// Returns [`None`] when no cache is configured, or when the archive isn't
	/// cached.
	pub async fn get_cached_archive(
		&self,
		build_hash: &str,
	) -> Result<Option<Vec<u8>>> {
		let Some(conn) = &self.redis else {
			return Ok(None);
		};
		let mut conn = conn.clone();
		let archive = conn
			.get::<_, Option<Vec<u8>>>(format!(
				"{ARCHIVE_KEY_PREFIX}{build_hash}"
			))
			.await
			.context("Failed to get cached archive from redis")?;
		Ok(archive)
	}

	/// Does nothing when no cache is configured.
	pub async fn cache_archive(
		&self,
		build_hash: &str,
		archive: &[u8],
	) -> Result<()> {
		let Some(conn) = &self.redis else {
			return Ok(());
		};
		// cheap, every clone shares the one connection
		let mut conn = conn.clone();
		conn.set_ex::<_, _, ()>(
			format!("{ARCHIVE_KEY_PREFIX}{build_hash}"),
			archive,
			ARCHIVE_TTL,
		)
		.await
		.context("Failed to cache archive in redis")?;
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
