use std::sync::OnceLock;

#[derive(Debug, Default, Clone)]
pub struct Value<T: Copy>(OnceLock<T>);

const _: () = {
	const fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<Value<()>>();
	assert_send_sync::<Ref<()>>();
};

impl<T: Copy> Value<T> {
	pub fn get<F: FnOnce() -> T>(&self, f: F) -> T {
		*self.0.get_or_init(f)
	}
}

#[derive(Debug, Clone)]
pub struct Ref<T>(OnceLock<T>);

impl<T> Ref<T> {
	pub const fn new() -> Self {
		Self(OnceLock::new())
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
