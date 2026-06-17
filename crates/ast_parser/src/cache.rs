use std::cell::OnceCell;

#[derive(Debug, Default, Clone)]
pub struct Value<T: Copy>(OnceCell<T>);

impl<T: Copy> Value<T> {
	pub fn get<F: FnOnce() -> T>(&self, f: F) -> T {
		*self.0.get_or_init(f)
	}
}

#[derive(Debug, Clone)]
pub struct Ref<T>(OnceCell<T>);

impl<T> Ref<T> {
	pub const fn new() -> Self {
		Self(OnceCell::new())
	}

	pub fn get<F: FnOnce() -> T>(&self, f: F) -> &T {
		self.0.get_or_init(f)
	}
}

impl<T: Default> Ref<T> {
	pub fn get_or_default<F: FnOnce() -> Option<T>>(&self, f: F) -> &T {
		self.0
			.get_or_init(|| f().unwrap_or_default())
	}
}

impl<T> Default for Ref<T> {
	fn default() -> Self {
		Self::new()
	}
}
