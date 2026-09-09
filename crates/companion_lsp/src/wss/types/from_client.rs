use crate::wss::{Semver, types::JsonNull};
use anyhow::{Result, anyhow};
use explorer_types::ModuleId;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// FIXME: cursed, but i don't think there's a better way to represent the js wire types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum FullMessage {
	/// This has to be first because untagged tries to match in declaration order,
	/// and `error` is never set in [`Self::Ok`]
	Err {
		#[serde(default, rename = "data")]
		_data: JsonNull,
		error: String,
		nonce: u32,
	},
	Ok {
		#[serde(flatten)]
		msg: IncomingMessage,
		nonce: u32,
	},
}

impl FullMessage {
	pub const fn nonce(&self) -> u32 {
		match self {
			Self::Ok { nonce, .. } | Self::Err { nonce, .. } => *nonce,
		}
	}
}

impl From<FullMessage> for Result<IncomingMessage, String> {
	fn from(value: FullMessage) -> Self {
		match value {
			FullMessage::Err {
				error,
				_data: _,
				nonce: _,
			} => Err(error),
			FullMessage::Ok { msg, nonce: _ } => Ok(msg),
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IncomingMessage {
	#[serde(rename = "diff")]
	DiffModule { data: DiffModule },
	#[serde(rename = "extract")]
	ExtractModule { data: ExtractModule },
	#[serde(rename = "moduleList")]
	ModuleList { data: ModuleList },
	#[serde(rename = "i18n")]
	IntlValue { data: IntlValue },
	#[serde(rename = "version")]
	Version { data: ClientVersion },
	#[serde(rename = "genericOk")]
	NotificationResponse {
		#[serde(flatten)]
		data: GenericOk,
	},
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericOk {
	pub data: (),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersion {
	pub client_version: Semver,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntlValue {
	pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModuleList {
	/// vec of stringified [`ModuleId`]
	pub modules: Vec<SmolStr>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtractModule {
	pub module: String,
	#[serde(default)]
	pub find: bool,
	#[serde(flatten)]
	pub module_result: ModuleResult,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffModule {
	pub source: String,
	pub patched: String,
	#[serde(flatten)]
	pub module_result: ModuleResult,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModuleResult {
	pub module_number: ModuleId,
	pub patched_by: Vec<String>,
}

macro_rules! impl_from_wire {
	($ty:ident) => {
		impl_from_wire!($ty, $ty);
	};
	($ty:ident, $var:ident) => {
		impl MsgFromClient for $ty {
			fn from_wire(msg: FullMessage) -> Result<Self> {
				match msg {
					FullMessage::Ok { msg, nonce: _ } => match msg {
						IncomingMessage::$var { data } => Ok(data),
						_ => Err(anyhow!(
							"Expected {ty} message, got {msg:?}",
							ty = stringify!($ty),
							msg = msg
						)),
					},
					FullMessage::Err {
						error,
						_data: _,
						nonce: _,
					} => Err(anyhow!("Client Error: {error}")),
				}
			}
		}
	};
}

impl_from_wire!(DiffModule);
impl_from_wire!(ExtractModule);
impl_from_wire!(ModuleList);
impl_from_wire!(IntlValue);
impl_from_wire!(ClientVersion, Version);
impl_from_wire!(GenericOk, NotificationResponse);

impl MsgFromClient for () {
	fn from_wire(_: FullMessage) -> Result<Self> {
		unreachable!("deserializing marker type");
	}
	fn notification_type() -> Option<Self> {
		Some(())
	}
}

pub trait MsgFromClient: Sized {
	fn from_wire(msg: FullMessage) -> Result<Self>;
	/// used for the marker type [`()`] for messages **to** the client that don't expect a response
	fn notification_type() -> Option<Self> {
		None
	}
}

/// Asserts that we accept the wire shape of every message the plugin sends
#[cfg(test)]
mod wire_shape {
	use serde_json::json;

	use crate::JValue;

	use super::*;

	/// Parses a frame the way [`crate::wss::WsServer`] does, from the text
	/// the plugin puts on the wire.
	fn parse(value: &JValue) -> FullMessage {
		let text = serde_json::to_string(value).expect("valid json");
		serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e} in {text}"))
	}

	fn parse_as<T: MsgFromClient>(value: &JValue) -> Result<T> {
		T::from_wire(parse(value))
	}

	/// `DiffModule.data = { source, patched } & ModuleResult`
	#[test]
	fn diff_module() {
		let msg = parse_as::<DiffModule>(&json!({
			"ok": true,
			"nonce": 1,
			"type": "diff",
			"data": {
				"source": "before",
				"patched": "after",
				"moduleNumber": 42,
				"patchedBy": ["SomePlugin"],
			},
		}))
		.expect("should deserialize");
		assert_eq!(msg.source, "before");
		assert_eq!(msg.patched, "after");
		assert_eq!(msg.module_result.module_number, ModuleId(42));
		assert_eq!(msg.module_result.patched_by, ["SomePlugin"]);
	}

	/// `ExtractModule.data = { module, find? } & ModuleResult`
	#[test]
	fn extract_module() {
		let msg = parse_as::<ExtractModule>(&json!({
			"ok": true,
			"nonce": 2,
			"type": "extract",
			"data": {
				"module": "0:function(){}",
				"find": true,
				"moduleNumber": 7,
				"patchedBy": [],
			},
		}))
		.expect("should deserialize");
		assert_eq!(msg.module, "0:function(){}");
		assert!(msg.find);
		assert_eq!(msg.module_result.module_number, ModuleId(7));
		assert_eq!(msg.module_result.patched_by, Vec::<String>::new());
	}

	/// `find` is optional on `ExtractModule.data`
	#[test]
	fn extract_module_without_find() {
		let msg = parse_as::<ExtractModule>(&json!({
			"ok": true,
			"nonce": 3,
			"type": "extract",
			"data": {
				"module": "0:function(){}",
				"moduleNumber": 7,
				"patchedBy": [],
			},
		}))
		.expect("should deserialize");
		assert!(!msg.find);
	}

	/// `ModuleList = { type: "moduleList", data: { modules } }`
	#[test]
	fn module_list() {
		let msg = parse_as::<ModuleList>(&json!({
			"ok": true,
			"nonce": 4,
			"type": "moduleList",
			"data": { "modules": ["1", "2", "3"] },
		}))
		.expect("should deserialize");
		assert_eq!(msg.modules, ["1", "2", "3"]);
	}

	/// `I18nValue = { type: "i18n", data: { value } }`
	#[test]
	fn i18n_value() {
		let msg = parse_as::<IntlValue>(&json!({
			"ok": true,
			"nonce": 5,
			"type": "i18n",
			"data": { "value": "Hello" },
		}))
		.expect("should deserialize");
		assert_eq!(msg.value, "Hello");
	}

	/// `VersionResponse = { type: "version", data: { clientVersion } }`
	#[test]
	fn version_response() {
		let msg = parse_as::<ClientVersion>(&json!({
			"ok": true,
			"nonce": 6,
			"type": "version",
			"data": { "clientVersion": [0, 1, 3] },
		}))
		.expect("should deserialize");
		assert_eq!(msg.client_version, (0, 1, 3));
	}

	/// `GenericOk = { type: "genericOk", data: {} }` — `data` is an empty
	/// *object*, not `null`
	#[test]
	fn generic_ok() {
		parse_as::<GenericOk>(&json!({
			"ok": true,
			"nonce": 7,
			"type": "genericOk",
			"data": {},
		}))
		.expect("should deserialize");
	}

	/// The `ok: false` half of `Base<T>` keeps `type` and adds `error`
	#[test]
	fn error_payload() {
		let wire = parse(&json!({
			"ok": false,
			"nonce": 8,
			"type": "diff",
			"data": null,
			"error": "module not found",
		}));
		assert_eq!(wire.nonce(), 8);
		assert!(matches!(wire, FullMessage::Err { .. }));
		let err = DiffModule::from_wire(wire)
			.expect_err("an error payload is not a DiffModule");
		assert_eq!(err.to_string(), "Client Error: module not found");
	}

	/// `Nonce` rides along on every frame, and dispatch depends on reading it
	/// back off both halves of `Base<T>`
	#[test]
	fn nonce_is_preserved() {
		assert_eq!(
			parse(&json!({
				"ok": true,
				"nonce": 9,
				"type": "i18n",
				"data": { "value": "Hello" },
			}))
			.nonce(),
			9
		);
	}

	/// A payload of the wrong type is a mismatch, not a parse failure
	#[test]
	fn wrong_message_type() {
		parse_as::<ModuleList>(&json!({
			"ok": true,
			"nonce": 10,
			"type": "i18n",
			"data": { "value": "Hello" },
		}))
		.expect_err("i18n is not a moduleList");
	}
}
