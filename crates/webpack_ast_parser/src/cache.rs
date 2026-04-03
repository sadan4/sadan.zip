use std::cell::OnceCell;

#[derive(Debug, Default, Clone)]
pub struct CacheValue<T: Copy>(OnceCell<T>);

impl<T: Copy> CacheValue<T> {
	pub fn get<F: FnOnce() -> T>(&self, f: F) -> T {
		*self.0.get_or_init(f)
	}
}

#[derive(Debug, Clone)]
pub struct CacheRef<T>(OnceCell<T>);

impl<T> CacheRef<T> {
	pub const fn new() -> Self {
		Self(OnceCell::new())
	}

	pub fn get<F: FnOnce() -> T>(&self, f: F) -> &T {
		self.0.get_or_init(f)
	}

}

impl<T: Default> CacheRef<T> {
	pub fn get_or_default<F: FnOnce() -> Option<T>>(&self, f: F) -> &T {
		self.0.get_or_init(|| f().unwrap_or_default())
	}
}


impl<T> Default for CacheRef<T> {
	fn default() -> Self {
		Self::new()
	}
}
