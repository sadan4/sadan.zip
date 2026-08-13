use std::ops::Index;

pub struct Mapping(pub(crate) &'static [usize]);

#[macro_export]
macro_rules! mapping {
	($name:ident, $($nums:literal),+) => {
		pub const $name: $crate::mapping::Mapping = $crate::mapping::Mapping(&[$($nums),+]);
	}
}

impl Index<Mapping> for serde_json::Value {
	type Output = serde_json::Value;

	fn index(&self, index: Mapping) -> &Self::Output {
		let mut result = self;
		for i in index.0 {
			result = &result[*i];
		}
		result
	}
}
