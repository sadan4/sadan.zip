use std::{any::Any, collections::HashMap, hash::BuildHasher, ops::Index};

use smol_str::{SmolStr, format_smolstr};

pub trait DowncastableIndex: Any + serde_json::value::Index {}

impl<T: Any + serde_json::value::Index> DowncastableIndex for T {}

#[derive(Clone, Copy)]
pub struct Mapping(pub(crate) &'static [&'static dyn DowncastableIndex]);

impl Mapping {
	fn slice_first(self) -> Self {
		Self(&self.0[1..])
	}
}

#[macro_export]
macro_rules! mapping {
	($name:ident, $($nums:literal),+) => {
		pub const $name: $crate::mapping::Mapping = $crate::mapping::Mapping(&[$(&$nums),+]);
	};
}

impl Index<Mapping> for serde_json::Value {
	type Output = Self;

	fn index(&self, this: Mapping) -> &Self::Output {
		let mut result = self;
		for i in this.0 {
			result = &result[*i];
		}
		result
	}
}

impl<S> Index<Mapping> for HashMap<SmolStr, serde_json::Value, S>
where
	S: BuildHasher,
{
	type Output = serde_json::Value;

	fn index(&self, this: Mapping) -> &Self::Output {
		let k1 = this.0[0] as &dyn Any;
		&if let Some(k_str) = k1.downcast_ref::<&str>() {
			self.get(*k_str)
		} else if let Some(k_string) = k1.downcast_ref::<String>() {
			self.get(k_string.as_str())
		} else if let Some(k_num) = k1.downcast_ref::<usize>() {
			let ss = format_smolstr!("{}", k_num);

			self.get(ss.as_str())
		} else {
			unreachable!("unhandled downcast")
		}
		.unwrap_or(&serde_json::Value::Null)[this.slice_first()]
	}
}

mapping!(_TEST_TYPE_CHECK_1, 0, 1, 2);
mapping!(_TEST_TYPE_CHECK_2, 0);
mapping!(_TEST_TYPE_CHECK_3, "str", 1, 2);
mapping!(_TEST_TYPE_CHECK_4, "str");

#[cfg(test)]
mod tests {
	use std::{
		collections::{HashMap, hash_map::DefaultHasher},
		hash::BuildHasherDefault,
		iter,
	};

	use serde_json::{Value as JV, json};
	use smol_str::SmolStr;

	use super::{DowncastableIndex, Mapping};

	/// Build a [`Mapping`] from a slice literal, coercing the elements to
	/// trait objects.
	const fn m(parts: &'static [&'static dyn DowncastableIndex]) -> Mapping {
		Mapping(parts)
	}

	/// Build a [`Mapping`] whose first key is a [`String`] (rather than a
	/// `&'static str`), to exercise the `String` downcast branch.
	fn string_key(key: &str) -> Mapping {
		let key: &'static String = Box::leak(Box::new(key.to_owned()));

		Mapping(Box::leak(Box::new([key as &dyn DowncastableIndex])))
	}

	/// Turn a JSON object into the map shape the impl is written against.
	fn map(value: &JV) -> HashMap<SmolStr, JV> {
		value
			.as_object()
			.expect("test input should be a JSON object")
			.iter()
			.map(|(k, v)| (SmolStr::new(k), v.clone()))
			.collect()
	}

	mapping!(MACRO_MAPPING, "a", 1);

	#[test]
	fn str_key_only() {
		let map = map(&json!({ "a": 1 }));

		assert_eq!(map[m(&[&"a"])], json!(1));
	}

	#[test]
	fn str_key_then_nested_lookups() {
		let map = map(&json!({ "a": { "b": [10, 20] } }));

		assert_eq!(map[m(&[&"a", &"b", &1usize])], json!(20));
	}

	#[test]
	fn string_key_downcast() {
		let map = map(&json!({ "a": "hi" }));

		assert_eq!(map[string_key("a")], json!("hi"));
	}

	/// A leading `usize` indexes the map by the *stringified* number, while
	/// later `usize`s index into arrays as normal.
	#[test]
	fn usize_key_is_stringified() {
		let map = map(&json!({ "0": [7, 8] }));

		assert_eq!(map[m(&[&0usize])], json!([7, 8]));
		assert_eq!(map[m(&[&0usize, &1usize])], json!(8));
	}

	#[test]
	fn macro_built_mapping() {
		let map = map(&json!({ "a": ["x", "y"] }));

		assert_eq!(map[MACRO_MAPPING], json!("y"));
	}

	#[test]
	fn missing_key_is_null() {
		let map = map(&json!({ "a": 1 }));

		assert_eq!(map[m(&[&"nope"])], JV::Null);
		assert_eq!(map[m(&[&"nope", &"deeper", &0usize])], JV::Null);
		assert_eq!(map[m(&[&12usize])], JV::Null);
	}

	#[test]
	fn missing_nested_key_is_null() {
		let map = map(&json!({ "a": { "b": 1 } }));

		assert_eq!(map[m(&[&"a", &"missing"])], JV::Null);
	}

	#[test]
	fn mismatched_index_kind_is_null() {
		let map = map(&json!({ "arr": [1, 2], "obj": { "k": 1 } }));

		// indexing an array by string, and an object by number
		assert_eq!(map[m(&[&"arr", &"k"])], JV::Null);
		assert_eq!(map[m(&[&"obj", &0usize])], JV::Null);
		// indexing past a scalar
		assert_eq!(map[m(&[&"arr", &0usize, &0usize])], JV::Null);
	}

	#[test]
	fn empty_map_is_null() {
		let map: HashMap<SmolStr, JV> = HashMap::new();

		assert_eq!(map[m(&[&"a"])], JV::Null);
	}

	/// The impl is generic over the hasher, not tied to [`RandomState`].
	///
	/// [`RandomState`]: std::collections::hash_map::RandomState
	#[test]
	fn works_with_any_hasher() {
		let map: HashMap<SmolStr, JV, BuildHasherDefault<DefaultHasher>> =
			iter::once((SmolStr::new("a"), json!({ "b": 1 }))).collect();

		assert_eq!(map[m(&[&"a", &"b"])], json!(1));
	}
}
