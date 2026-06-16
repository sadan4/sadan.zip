//! Wire-format types for the WebSocket protocol between this server and the
//! `vc-userDevTools` Discord client. The shapes here are a direct port of
//! `VencordCompanion/src/types/server/{send,recieve}.ts` — *do not* change
//! field names or shapes without also updating the userplugin, since older
//! Discord plugin builds will silently malfunction.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Common atoms
// ---------------------------------------------------------------------------

pub type SemVer = (u32, u32, u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FindNode {
	#[serde(rename = "string")]
	String { value: String },
	#[serde(rename = "regex")]
	Regex { value: RegexValue },
	#[serde(rename = "function")]
	Function { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexValue {
	pub pattern: String,
	pub flags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MatchNode {
	#[serde(rename = "string")]
	String { value: String },
	#[serde(rename = "regex")]
	Regex { value: RegexValue },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ReplaceNode {
	#[serde(rename = "string")]
	String { value: String },
	#[serde(rename = "function")]
	Function { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
	#[serde(rename = "match")]
	pub match_: MatchNode,
	pub replace: ReplaceNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchData {
	pub find_type: FindType,
	pub find: String,
	pub replacement: Vec<Replacement>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindType {
	String,
	Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindData {
	/// e.g. `findComponent`, `findByPropsLazy`, `findByCode`, etc.
	#[serde(rename = "type")]
	pub kind: String,
	pub args: Vec<FindNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisablePluginData {
	pub enabled: bool,
	pub plugin_name: String,
}

// SearchData / FindOrSearchData are kept as `serde_json::Value` for now —
// they are command-side payloads constructed by `lsp::commands`, which
// already speaks JSON. Modeling them strictly buys little until those
// handlers exist.

// ---------------------------------------------------------------------------
// Outgoing (server -> Discord client)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OutgoingKind {
	Disable { data: DisablePluginData },
	Diff { data: serde_json::Value },
	Reload { data: () },
	Extract { data: serde_json::Value },
	TestPatch { data: PatchData },
	TestFind { data: FindData },
	AllModules { data: () },
	I18n { data: I18nLookupData },
	Version { data: VersionRequestData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct I18nLookupData {
	pub hashed_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRequestData {
	/// Keep the `snake_case` JSON name verbatim — matches `vc-userDevTools`.
	pub server_version: SemVer,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutgoingFrame {
	#[serde(flatten)]
	pub kind: OutgoingKind,
	pub nonce: u64,
}

// ---------------------------------------------------------------------------
// Incoming (Discord client -> server)
// ---------------------------------------------------------------------------

/// Envelope used for the wire. Deserialized loosely: `data` is kept as a
/// `serde_json::Value` and refined into a typed payload via `IncomingKind`.
///
/// `kind` is `#[serde(default)]` because the Discord client omits `"type"`
/// from nonce-correlated error responses (`{nonce, ok:false, error}`). It's
/// only meaningful for unsolicited frames like `moduleList`; for routed
/// responses the nonce is the discriminator.
#[derive(Debug, Clone, Deserialize)]
pub struct IncomingFrame {
	#[serde(rename = "type", default)]
	pub kind: String,
	#[serde(default)]
	pub nonce: Option<u64>,
	pub ok: bool,
	#[serde(default)]
	pub data: serde_json::Value,
	#[serde(default)]
	pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffModuleData {
	pub source: String,
	pub patched: String,
	pub module_number: i64,
	#[serde(default)]
	pub patched_by: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractModuleData {
	pub module: String,
	#[serde(default)]
	pub find: Option<bool>,
	pub module_number: i64,
	#[serde(default)]
	pub patched_by: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleListData {
	pub modules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct I18nValueData {
	pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionResponseData {
	pub client_version: SemVer,
}

impl IncomingFrame {
	/// Surface the Discord-side error message verbatim. We intentionally do
	/// not prepend any framing — the message ends up directly in user-facing
	/// diagnostics and toasts, where a `"discord client returned error for X:"`
	/// prefix is just noise (and looks especially broken for nonce-correlated
	/// error frames where `kind` is empty).
	pub fn into_error(self) -> anyhow::Error {
		anyhow::anyhow!(
			"{}",
			self.error
				.as_deref()
				.unwrap_or("Discord client returned an unspecified error"),
		)
	}

	pub fn parse_data<T: for<'de> Deserialize<'de>>(
		&self,
	) -> anyhow::Result<T> {
		serde_json::from_value(self.data.clone()).map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn outgoing_test_patch_round_trips() {
		let frame = OutgoingFrame {
			kind: OutgoingKind::TestPatch {
				data: PatchData {
					find_type: FindType::Regex,
					find: "/foo/".into(),
					replacement: vec![Replacement {
						match_: MatchNode::Regex {
							value: RegexValue {
								pattern: "a".into(),
								flags: "g".into(),
							},
						},
						replace: ReplaceNode::String { value: "b".into() },
					}],
				},
			},
			nonce: 42,
		};
		let v = serde_json::to_value(&frame).unwrap();
		assert_eq!(v["type"], "testPatch");
		assert_eq!(v["nonce"], 42);
		assert_eq!(v["data"]["findType"], "regex");
		assert_eq!(v["data"]["replacement"][0]["match"]["type"], "regex");
		assert_eq!(v["data"]["replacement"][0]["replace"]["value"], "b");
	}

	#[test]
	fn outgoing_reload_serializes_data_null() {
		let frame = OutgoingFrame {
			kind: OutgoingKind::Reload { data: () },
			nonce: 7,
		};
		let v = serde_json::to_value(&frame).unwrap();
		assert_eq!(v["data"], serde_json::Value::Null);
		assert_eq!(v["type"], "reload");
	}

	#[test]
	fn incoming_envelope_parses_ok_extract() {
		let raw = serde_json::json!({
			"type": "extract",
			"ok": true,
			"nonce": 9,
			"data": {
				"module": "function(){}",
				"moduleNumber": 12345,
				"patchedBy": ["FooPlugin"]
			}
		});
		let frame: IncomingFrame = serde_json::from_value(raw).unwrap();
		assert!(frame.ok);
		assert_eq!(frame.nonce, Some(9));
		assert_eq!(frame.kind, "extract");
		let typed: ExtractModuleData = frame.parse_data().unwrap();
		assert_eq!(typed.module_number, 12345);
		assert_eq!(typed.patched_by, vec!["FooPlugin"]);
	}

	#[test]
	fn incoming_envelope_parses_err() {
		let raw = serde_json::json!({
			"type": "testPatch",
			"ok": false,
			"nonce": 3,
			"data": null,
			"error": "match did not apply"
		});
		let frame: IncomingFrame = serde_json::from_value(raw).unwrap();
		assert!(!frame.ok);
		assert_eq!(frame.error.as_deref(), Some("match did not apply"));
		let err = frame.into_error();
		let s = err.to_string();
		assert_eq!(
			s, "match did not apply",
			"expected bare upstream error, got {s:?}"
		);
	}

	#[test]
	fn incoming_error_without_type_field() {
		// Real shape sent by vc-userDevTools for a failed testPatch — no
		// `type` field. Regression for the nonce 8486 frame that was getting
		// dropped on the floor and timing out.
		let raw = serde_json::json!({
			"nonce": 8486,
			"ok": false,
			"error": "Replacement 1 failed: SyntaxError: Unexpected token ')'"
		});
		let frame: IncomingFrame = serde_json::from_value(raw).unwrap();
		assert!(!frame.ok);
		assert_eq!(frame.nonce, Some(8486));
		assert_eq!(frame.kind, "");
		assert!(
			frame
				.error
				.as_deref()
				.unwrap()
				.contains("Replacement 1")
		);
	}

	#[test]
	fn incoming_unsolicited_module_list_has_no_nonce() {
		let raw = serde_json::json!({
			"type": "moduleList",
			"ok": true,
			"data": { "modules": ["1", "2", "3"] }
		});
		let frame: IncomingFrame = serde_json::from_value(raw).unwrap();
		assert_eq!(frame.nonce, None);
		let typed: ModuleListData = frame.parse_data().unwrap();
		assert_eq!(typed.modules.len(), 3);
	}

	#[test]
	fn version_request_uses_snake_case_field() {
		let frame = OutgoingFrame {
			kind: OutgoingKind::Version {
				data: VersionRequestData {
					server_version: (1, 2, 3),
				},
			},
			nonce: 1,
		};
		let v = serde_json::to_value(&frame).unwrap();
		// Keep the TS protocol's field name verbatim:
		assert_eq!(v["data"]["server_version"], serde_json::json!([1, 2, 3]));
	}
}
