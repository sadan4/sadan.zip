pub mod ast;
use std::{collections::HashMap, fmt, sync::LazyLock};

use anyhow::Result;
use serde_json::{Map, Value};
use smol_str::SmolStr;

static KEY_MAPPINGS_MPK_ZST: &[u8] = include_bytes!("./key_mappings.mpk.zst");

/// Parsed, lazily-initialised map of hashed key -> unhashed message name.
static KEY_MAPPINGS: LazyLock<HashMap<SmolStr, SmolStr>> =
	LazyLock::new(|| {
		let raw = zstd::Decoder::new(KEY_MAPPINGS_MPK_ZST)
			.expect("Failed to decompress key_mappings.mpk.zst");
		rmp_serde::from_read(raw).expect("Failed to parse key_mappings.mpk.zst")
	});

/// Attempt to resolve a hashed i18n key to its original (unhashed) message
/// name. Returns `None` when the key is not present in the mapping.
pub fn resolve_unhashed_key(hashed: &str) -> Option<SmolStr> {
	KEY_MAPPINGS.get(hashed).cloned()
}

pub fn render_message(
	to: &mut dyn fmt::Write,
	message: &[ast::FormatJsNode],
	format_args: &Map<String, Value>,
) -> Result<()> {
	fn render_node(
		to: &mut dyn fmt::Write,
		node: &ast::FormatJsNode,
		format_args: &Map<String, Value>,
	) -> Result<()> {
		match node {
			ast::FormatJsNode::Literal(lit) => write!(to, "{lit}")?,
			ast::FormatJsNode::Argument(arg) => match format_args.get(arg) {
				Some(Value::String(s)) => write!(to, "{s}")?,
				Some(Value::Number(n)) => write!(to, "{n}")?,
				Some(Value::Bool(b)) => write!(to, "{b}")?,
				Some(_) => {
					write!(to, "[unsupported value type for argument '{arg}']")?;
				}
				None => write!(to, "[<{arg}>]")?,
			},
			ast::FormatJsNode::Number { value, style: _ } => {
				write!(to, "[number <{value}>]")?;
			}
			ast::FormatJsNode::Date { value, style: _ } => {
				write!(to, "[date <{value}>]")?;
			}
			ast::FormatJsNode::Time { value, style: _ } => {
				write!(to, "[time <{value}>]")?;
			}
			ast::FormatJsNode::Select { value, .. } => {
				write!(to, "[select <{value}>]")?;
			}
			ast::FormatJsNode::Plural { value, .. } => {
				write!(to, "[plural <{value}>]")?;
			}
			ast::FormatJsNode::Pound => {
				write!(to, "#")?;
			}
			ast::FormatJsNode::Tag { .. } => {
				write!(to, "[tag node <TODO>]")?;
			}
		}
		Ok(())
	}
	for node in message {
		render_node(to, node, format_args)?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_key_mappings() {
		LazyLock::force(&KEY_MAPPINGS);
	}
}
