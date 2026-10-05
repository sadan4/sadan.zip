use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use typesize::derive::TypeSize;

use crate::{ModuleId, size_serde_json_value};

/// Every english intl message in a build, keyed by its hashed key
///
/// Stored next to the build's data file instead of inside [`FullBundle`],
/// so looking up a message never has to decompress the whole bundle, and
/// backfilling it never has to rewrite the bundle
///
/// [`FullBundle`]: crate::FullBundle
#[derive(
	Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, TypeSize,
)]
#[serde(rename_all = "camelCase")]
pub struct IntlMessages {
	/// sorted ids of every english intl module in the build
	pub modules: Vec<ModuleId>,
	/// hashed keys are 6 chars, so [`SmolStr`] keeps them inline
	#[typesize(with = size_intl_messages)]
	pub messages: HashMap<SmolStr, IntlMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TypeSize)]
#[serde(rename_all = "camelCase")]
pub struct IntlMessage {
	/// the module whose JSON defines this message
	pub module: ModuleId,
	/// the raw intl AST json
	#[typesize(with = size_serde_json_value)]
	pub value: serde_json::Value,
}

impl IntlMessages {
	pub fn shrink_to_fit(&mut self) {
		let Self { modules, messages } = self;
		modules.shrink_to_fit();
		messages.shrink_to_fit();
	}
}

/// typesize has no impl for [`SmolStr`]
fn size_intl_messages(messages: &HashMap<SmolStr, IntlMessage>) -> usize {
	let entries = messages.capacity() * size_of::<(SmolStr, IntlMessage)>();
	let heap: usize = messages
		.iter()
		.map(|(k, v)| {
			let key = if k.is_heap_allocated() { k.len() } else { 0 };
			// the `Value` itself is already counted inline in `entries`
			key + size_serde_json_value(&v.value)
				- size_of::<serde_json::Value>()
		})
		.sum();
	size_of::<HashMap<SmolStr, IntlMessage>>() + entries + heap
}
