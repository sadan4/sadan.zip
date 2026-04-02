use std::cell::OnceCell;

#[derive(Debug, Default, Clone)]
pub struct CacheValue<T: Copy>(OnceCell<T>);

impl<T: Copy> CacheValue<T> {
	pub fn get<F: FnOnce() -> T>(&self, f: F) -> T {
		*self.0.get_or_init(f)
	}
}
