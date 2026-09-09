use explorer_types::ModuleId;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::wss::types::JsonNull;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FullMessage {
	pub nonce: u32,
	#[serde(flatten)]
	pub msg: Message,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct ReloadMessage {
	pub data: JsonNull,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct LoadModulesMessage {
	pub data: JsonNull,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffMessage {
	#[serde(flatten)]
	pub data: Query,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtractMessage {
	#[serde(flatten)]
	pub data: FindQuery,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextPatchMessage {
	#[serde(flatten)]
	pub find: PatchFind,
	#[serde(rename = "replacement")]
	pub replace: Vec<PatchReplacement>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestFindMessage {
	#[serde(flatten)]
	pub find: FindData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionMessage {
	#[serde(rename = "server_version")]
	pub server_version: (u16, u16, u16),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
	Disable {
		data: DisableMessage,
	},
	#[serde(rename = "diff")]
	DiffPatch {
		data: DiffMessage,
	},
	/// Reloads the client, this will discard the client and all it's state
	Reload {
		#[serde(flatten)]
		data: ReloadMessage,
	},
	#[serde(rename = "extract")]
	ExtractModule {
		data: ExtractMessage,
	},
	#[serde(rename = "testPatch")]
	TestPatch {
		data: TextPatchMessage,
	},
	#[serde(rename = "testFind")]
	TestFind {
		data: TestFindMessage,
	},
	#[serde(rename = "allModules")]
	/// A message sent to the server to instruct it to load all modules
	AllModules {
		#[serde(flatten)]
		data: LoadModulesMessage,
	},
	#[serde(rename = "i18n")]
	IntlLookup {
		data: IntlLookup,
	},
	#[serde(rename = "version")]
	Version {
		data: VersionMessage,
	},
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntlLookup {
	pub hashed_key: SmolStr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PatchReplacement {
	#[serde(rename = "match")]
	pub match_: ReplaceNode,
	pub replace: ReplaceNode,
}

/// The `value` of a `RegexNode`, which nests its parts instead of carrying
/// them alongside the `type` tag
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegexValue {
	pub pattern: String,
	pub flags: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ReplaceNode {
	String { value: String },
	Regex { value: RegexValue },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "findType", rename_all = "camelCase")]
pub enum PatchFind {
	String {
		#[serde(rename = "find")]
		string: String,
	},
	Regex {
		/// Stringified regex
		#[serde(rename = "find")]
		regex: String,
	},
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "extractType", rename_all = "camelCase")]
pub enum Query {
	#[serde(rename = "id")]
	Id {
		#[serde(rename = "idOrSearch")]
		id: ModuleId,
	},
	#[serde(rename = "search")]
	Search {
		#[serde(flatten)]
		search: Search,
	},
}

/// `FindOrSearchData`: a [`Query`] plus `usePatched`, or a find.
///
/// The discriminant is `extractType`, same as [`Query`]; `findType` is the
/// inner selector supplied by the flattened [`Search`].
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "extractType", rename_all = "camelCase")]
pub enum FindQuery {
	#[serde(rename = "id")]
	Id {
		#[serde(rename = "idOrSearch")]
		id: ModuleId,
		#[serde(rename = "usePatched")]
		use_patched: bool,
	},
	#[serde(rename = "search")]
	Search {
		#[serde(flatten)]
		search: Search,
		/// if None, uses the client default setting
		#[serde(rename = "usePatched")]
		use_patched: Option<bool>,
	},
	#[serde(rename = "find")]
	Find(PrefixedFindData),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FindData {
	#[serde(rename = "type")]
	pub find_type: String,
	pub args: Vec<FindNode>,
}

/// [`FindData`] with its keys capitalized and prefixed with `find`, which is
/// how it appears inside `FindOrSearchData` (but *not* inside `TestFind`)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrefixedFindData {
	#[serde(rename = "findType")]
	pub find_type: String,
	#[serde(rename = "findArgs")]
	pub args: Vec<FindNode>,
}

impl From<FindData> for PrefixedFindData {
	fn from(FindData { find_type, args }: FindData) -> Self {
		Self { find_type, args }
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "findType", rename_all = "camelCase")]
pub enum Search {
	#[serde(rename = "string")]
	String {
		#[serde(rename = "idOrSearch")]
		string: String,
	},
	#[serde(rename = "regex")]
	Regex {
		#[serde(rename = "idOrSearch")]
		/// Stringified regex
		regex: String,
	},
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FindNode {
	String {
		value: String,
	},
	Regex {
		value: RegexValue,
	},
	Function {
		#[serde(rename = "value")]
		body: String,
	},
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DisableMessage {
	pub enabled: bool,
	pub plugin_name: SmolStr,
}

macro_rules! impl_to_client {
	($remote:ty, $ty:ident) => {
		impl_to_client!($remote, $ty, $ty);
	};
	($remote:ty, $ty:ident, $var:ident) => {
		impl_to_client!($remote, $ty, $var, data);
	};
	($remote:ty, $ty:ident, $var:ident, $field:ident) => {
		impl MsgToClient for $ty {
			fn to_wire(self, nonce: u32) -> FullMessage {
				FullMessage {
					nonce,
					msg: Message::$var { $field: self },
				}
			}
			type Response = $remote;
		}
	};
}

impl_to_client!((), DisableMessage, Disable);
impl_to_client!(super::from_client::DiffModule, DiffMessage, DiffPatch);
impl_to_client!((), ReloadMessage, Reload);
impl_to_client!(
	super::from_client::ExtractModule,
	ExtractMessage,
	ExtractModule
);
impl_to_client!(super::from_client::GenericOk, TextPatchMessage, TestPatch);
impl_to_client!(super::from_client::GenericOk, TestFindMessage, TestFind);
impl_to_client!(
	super::from_client::ModuleList,
	LoadModulesMessage,
	AllModules
);
impl_to_client!(super::from_client::IntlValue, IntlLookup);
impl_to_client!(super::from_client::ClientVersion, VersionMessage, Version);

pub trait MsgToClient {
	fn to_wire(self, nonce: u32) -> FullMessage;
	type Response: super::MsgFromClient;
}

/// Asserts the wire shape of every message we send against the original typescript types
#[cfg(test)]
mod wire_shape {
	use serde_json::json;

	use crate::JValue;

use super::*;

	/// Serializes `msg` exactly like [`crate::wss::WsServer::send_msg`] does
	/// (through [`serde_json::to_string`]) and reparses it, rejecting
	/// duplicate object keys.
	fn wire<T: MsgToClient>(msg: T, nonce: u32) -> JValue {
		let text = serde_json::to_string(&msg.to_wire(nonce))
			.expect("message should serialize");
		let NoDupJson(value) = serde_json::from_str(&text)
			.unwrap_or_else(|e| panic!("{e} in {text}"));
		value
	}

	/// A parsed JSON tree that rejects duplicate object keys.
	///
	/// Both [`JValue`] and `JSON.parse` silently keep the last of a
	/// duplicate pair, so a message carrying two `type` keys would otherwise
	/// compare equal to a correct one.
	struct NoDupJson(JValue);

	impl<'de> Deserialize<'de> for NoDupJson {
		fn deserialize<D>(des: D) -> Result<Self, D::Error>
		where
			D: serde::Deserializer<'de>,
		{
			des.deserialize_any(NoDupVisitor)
		}
	}

	struct NoDupVisitor;

	impl<'de> serde::de::Visitor<'de> for NoDupVisitor {
		type Value = NoDupJson;

		fn expecting(
			&self,
			f: &mut std::fmt::Formatter<'_>,
		) -> std::fmt::Result {
			f.write_str("any JSON value")
		}

		fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
			Ok(NoDupJson(JValue::Null))
		}

		fn visit_bool<E: serde::de::Error>(
			self,
			v: bool,
		) -> Result<Self::Value, E> {
			Ok(NoDupJson(v.into()))
		}

		fn visit_i64<E: serde::de::Error>(
			self,
			v: i64,
		) -> Result<Self::Value, E> {
			Ok(NoDupJson(v.into()))
		}

		fn visit_u64<E: serde::de::Error>(
			self,
			v: u64,
		) -> Result<Self::Value, E> {
			Ok(NoDupJson(v.into()))
		}

		fn visit_f64<E: serde::de::Error>(
			self,
			v: f64,
		) -> Result<Self::Value, E> {
			Ok(NoDupJson(v.into()))
		}

		fn visit_str<E: serde::de::Error>(
			self,
			v: &str,
		) -> Result<Self::Value, E> {
			Ok(NoDupJson(v.into()))
		}

		fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
		where
			A: serde::de::SeqAccess<'de>,
		{
			let mut out = Vec::new();
			while let Some(NoDupJson(v)) = seq.next_element()? {
				out.push(v);
			}
			Ok(NoDupJson(JValue::Array(out)))
		}

		fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
		where
			A: serde::de::MapAccess<'de>,
		{
			let mut out = serde_json::Map::new();
			while let Some(key) = map.next_key::<String>()? {
				let NoDupJson(value) = map.next_value()?;
				if out.insert(key.clone(), value).is_some() {
					return Err(serde::de::Error::custom(format!(
						"duplicate object key {key:?}"
					)));
				}
			}
			Ok(NoDupJson(JValue::Object(out)))
		}
	}

	/// `DisablePlugin = { type: "disable", data: { enabled, pluginName } }`
	#[test]
	fn disable() {
		let msg = DisableMessage {
			enabled: false,
			plugin_name: "MyPlugin".into(),
		};
		assert_eq!(
			wire(msg, 1),
			json!({
				"nonce": 1,
				"type": "disable",
				"data": {
					"enabled": false,
					"pluginName": "MyPlugin",
				},
			})
		);
	}

	/// `DiffPatch = { type: "diff", data: SearchData }`, `extractType: "id"`
	#[test]
	fn diff_by_id() {
		let msg = DiffMessage {
			data: Query::Id { id: ModuleId(42) },
		};
		assert_eq!(
			wire(msg, 2),
			json!({
				"nonce": 2,
				"type": "diff",
				"data": {
					"extractType": "id",
					"idOrSearch": 42,
				},
			})
		);
	}

	/// `SearchData`, `extractType: "search"` + `findType: "string"`
	#[test]
	fn diff_by_string_search() {
		let msg = DiffMessage {
			data: Query::Search {
				search: Search::String {
					string: "needle".into(),
				},
			},
		};
		assert_eq!(
			wire(msg, 3),
			json!({
				"nonce": 3,
				"type": "diff",
				"data": {
					"extractType": "search",
					"idOrSearch": "needle",
					"findType": "string",
				},
			})
		);
	}

	/// `SearchData`, `extractType: "search"` + `findType: "regex"`
	#[test]
	fn diff_by_regex_search() {
		let msg = DiffMessage {
			data: Query::Search {
				search: Search::Regex {
					regex: "/needle/g".into(),
				},
			},
		};
		assert_eq!(
			wire(msg, 4),
			json!({
				"nonce": 4,
				"type": "diff",
				"data": {
					"extractType": "search",
					"idOrSearch": "/needle/g",
					"findType": "regex",
				},
			})
		);
	}

	/// `Reload = { type: "reload", data: null }`
	#[test]
	fn reload() {
		assert_eq!(
			wire(ReloadMessage::default(), 5),
			json!({
				"nonce": 5,
				"type": "reload",
				"data": null,
			})
		);
	}

	/// `AllModules = { type: "allModules", data: null }`
	#[test]
	fn all_modules() {
		assert_eq!(
			wire(LoadModulesMessage::default(), 6),
			json!({
				"nonce": 6,
				"type": "allModules",
				"data": null,
			})
		);
	}

	/// `ExtractModule = { type: "extract", data: FindOrSearchData }`, where
	/// `FindOrSearchData = SearchData & { usePatched: boolean | null }`
	#[test]
	fn extract_by_id() {
		let msg = ExtractMessage {
			data: FindQuery::Id {
				id: ModuleId(7),
				use_patched: true,
			},
		};
		assert_eq!(
			wire(msg, 7),
			json!({
				"nonce": 7,
				"type": "extract",
				"data": {
					"extractType": "id",
					"idOrSearch": 7,
					"usePatched": true,
				},
			})
		);
	}

	/// `FindOrSearchData` over a search, with the client default for
	/// `usePatched`
	#[test]
	fn extract_by_search() {
		let msg = ExtractMessage {
			data: FindQuery::Search {
				search: Search::String {
					string: "needle".into(),
				},
				use_patched: None,
			},
		};
		assert_eq!(
			wire(msg, 8),
			json!({
				"nonce": 8,
				"type": "extract",
				"data": {
					"extractType": "search",
					"idOrSearch": "needle",
					"findType": "string",
					"usePatched": null,
				},
			})
		);
	}

	/// `{ extractType: "find" } & _PrefixKeys<_CapitalizeKeys<FindData>,
	/// "find">`, i.e. `FindData`'s `type`/`args` become `findType`/`findArgs`
	#[test]
	fn extract_by_find() {
		let msg = ExtractMessage {
			data: FindQuery::Find(
				FindData {
					find_type: "findByProps".into(),
					args: vec![FindNode::String {
						value: "getUser".into(),
					}],
				}
				.into(),
			),
		};
		assert_eq!(
			wire(msg, 9),
			json!({
				"nonce": 9,
				"type": "extract",
				"data": {
					"extractType": "find",
					"findType": "findByProps",
					"findArgs": [
						{ "type": "string", "value": "getUser" },
					],
				},
			})
		);
	}

	/// `TestPatch.data` names the needle `find` for both `findType`s, and
	/// nests the regex node's `pattern`/`flags` under `value`
	#[test]
	fn test_patch_string_find() {
		let msg = TextPatchMessage {
			find: PatchFind::String {
				string: "needle".into(),
			},
			replace: vec![PatchReplacement {
				match_: ReplaceNode::Regex {
					value: RegexValue {
						pattern: "a(b)".into(),
						flags: "g".into(),
					},
				},
				replace: ReplaceNode::String { value: "$1".into() },
			}],
		};
		assert_eq!(
			wire(msg, 10),
			json!({
				"nonce": 10,
				"type": "testPatch",
				"data": {
					"findType": "string",
					"find": "needle",
					"replacement": [{
						"match": {
							"type": "regex",
							"value": {
								"pattern": "a(b)",
								"flags": "g",
							},
						},
						"replace": {
							"type": "string",
							"value": "$1",
						},
					}],
				},
			})
		);
	}

	/// Same, with the stringified-regex needle
	#[test]
	fn test_patch_regex_find() {
		let msg = TextPatchMessage {
			find: PatchFind::Regex {
				regex: "/needle/g".into(),
			},
			replace: vec![],
		};
		assert_eq!(
			wire(msg, 11),
			json!({
				"nonce": 11,
				"type": "testPatch",
				"data": {
					"findType": "regex",
					"find": "/needle/g",
					"replacement": [],
				},
			})
		);
	}

	/// `TestFind = { type: "testFind", data: FindData }` — `data` is a bare
	/// [`FindData`] here, *not* the prefixed `FindOrSearchData` form
	#[test]
	fn test_find() {
		let msg = TestFindMessage {
			find: FindData {
				find_type: "findStore".into(),
				args: vec![FindNode::Function {
					body: "m=>m.default".into(),
				}],
			},
		};
		assert_eq!(
			wire(msg, 12),
			json!({
				"nonce": 12,
				"type": "testFind",
				"data": {
					"type": "findStore",
					"args": [
						{ "type": "function", "value": "m=>m.default" },
					],
				},
			})
		);
	}

	/// `I18nLookup = { type: "i18n", data: { hashedKey } }`
	#[test]
	fn i18n_lookup() {
		let msg = IntlLookup {
			hashed_key: "aBcDeF".into(),
		};
		assert_eq!(
			wire(msg, 13),
			json!({
				"nonce": 13,
				"type": "i18n",
				"data": {
					"hashedKey": "aBcDeF",
				},
			})
		);
	}

	/// `Version = { type: "version", data: { server_version } }` — the one
	/// `snake_case` key in `recieve.ts`
	#[test]
	fn version() {
		let msg = VersionMessage {
			server_version: (2, 0, 0),
		};
		assert_eq!(
			wire(msg, 14),
			json!({
				"nonce": 14,
				"type": "version",
				"data": {
					"server_version": [2, 0, 0],
				},
			})
		);
	}

	/// `FindNode = StringNode | RegexNode | FunctionNode`
	#[test]
	fn find_nodes() {
		let msg = ExtractMessage {
			data: FindQuery::Find(
				FindData {
					find_type: "find".into(),
					args: vec![
						FindNode::String { value: "s".into() },
						FindNode::Regex {
							value: RegexValue {
								pattern: "p".into(),
								flags: "gi".into(),
							},
						},
						FindNode::Function {
							body: "m=>m".into(),
						},
					],
				}
				.into(),
			),
		};
		assert_eq!(
			wire(msg, 15)["data"]["findArgs"],
			json!([
				{ "type": "string", "value": "s" },
				{
					"type": "regex",
					"value": { "pattern": "p", "flags": "gi" },
				},
				{ "type": "function", "value": "m=>m" },
			])
		);
	}
}
