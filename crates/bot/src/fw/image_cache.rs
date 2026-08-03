use std::{fmt::Debug, sync::Arc, time::Duration};

use mini_moka::sync::Cache;
use serenity::all::UserId;
use tokio::sync::OnceCell;
use tracing::{error, warn};

use crate::util::Image;

type CacheEntry = Arc<OnceCell<Image>>;
/// Cheap to clone, internals are refcounted
#[derive(Debug, Clone)]
pub struct ImageCache {
	cache: Cache<UserId, CacheEntry>,
	/// Cache for url/unique string -> image.
	dl_cache: Cache<String, CacheEntry>,
}

fn new_cache() -> Cache<UserId, CacheEntry> {
	Cache::builder()
		.max_capacity(2048)
		.time_to_idle(Duration::from_mins(5))
		.build()
}

impl ImageCache {
	pub(super) fn new() -> Self {
		Self {
			cache: Cache::builder()
				.max_capacity(4096)
				.time_to_idle(Duration::from_mins(5))
				.build(),
			dl_cache: Cache::builder()
				.max_capacity(1024)
				.time_to_idle(Duration::from_mins(10))
				.time_to_live(Duration::from_hours(1))
				.build(),
		}
	}
	pub async fn launch_dl_for_user<F, E>(
		&self,
		fut: F,
		user: UserId,
		key: impl Into<Option<String>>,
	) -> Result<CacheEntry, E>
	where
		F: Future<Output = Result<Image, E>>,
		E: Debug,
	{
		if let Some(key) = key.into() {
			if let Some(entry) = self.dl_cache.get(&key) {
				// update user cache with this entry
				self.cache.insert(user, entry.clone());
				Ok(entry)
			} else {
				let new_entry = CacheEntry::default();
				self.dl_cache
					.insert(key.clone(), new_entry.clone());
				self.cache
					.insert(user, new_entry.clone());
				let res = fut.await;
				match res {
					Ok(entry) => {
						if let Err(e) = new_entry.set(entry) {
							warn!(
								?user,
								?key,
								"Failed to set image cache entry {e:?}"
							);
						}
						Ok(new_entry)
					}
					Err(e) => {
						error!(?user, ?key, "Failed to fetch image: {e:?}");
						// invalidate the cache entry
						self.dl_cache.invalidate(&key);
						self.cache.invalidate(&user);
						Err(e)
					}
				}
			}
		} else {
			let new_entry = CacheEntry::default();
			self.cache
				.insert(user, new_entry.clone());
			let res = fut.await;
			match res {
				Ok(entry) => {
					if let Err(e) = new_entry.set(entry) {
						warn!(?user, "Failed to set image cache entry {e:?}");
					}
					Ok(new_entry)
				}
				Err(e) => {
					error!("Failed to fetch image for user {}: {e:?}", user);
					// invalidate the cache entry
					self.cache.invalidate(&user);
					Err(e)
				}
			}
		}
	}
	pub fn update_user_entry(&self, user: UserId, image: Image) {
		let entry = Arc::new(OnceCell::new_with(Some(image)));
		self.cache.insert(user, entry);
	}
	pub fn get_user_entry(&self, user: UserId) -> Option<CacheEntry> {
		self.cache.get(&user)
	}
}
