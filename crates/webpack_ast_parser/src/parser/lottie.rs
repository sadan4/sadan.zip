use serde_json::Value;

use crate::WebpackAstParser;

/// Just a simple hurestic,
/// a full schema can be found here: <https://github.com/lottie/lottie-spec>
fn looks_like_lottie(v: &Value) -> bool {
	try {
		let v = v.as_object()?;
		const REQUIRED_NUM_KEYS: &[&str] = &["fr", "ip", "op", "w", "h"];
		for key in REQUIRED_NUM_KEYS {
			if !v.get(*key)?.is_number() {
				return false;
			}
		}
		if !v.get("v")?.is_string()
			|| !v.get("layers")?.is_array()
			|| v.get("assets")
				.is_none_or(|v| !v.is_array())
		{
			return false;
		}
		true
	}
	.unwrap_or(false)
}

impl WebpackAstParser<'_> {
	pub fn as_lottie_module(&self) -> Option<Value> {
		self.as_json_module()
			.and_then(|v| serde_json::from_str(v).ok())
			.filter(looks_like_lottie)
	}
}
