//! <https://raw.githubusercontent.com/discord/discord-intl/refs/heads/main/packages/intl-ast/index.ts>
use std::{collections::HashMap, str::FromStr};

struct NodeType;
impl NodeType {
	#![expect(non_upper_case_globals)]
	#[expect(unused)]
	pub const Literal: u8 = 0;
	pub const Argument: u8 = 1;
	pub const Number: u8 = 2;
	pub const Date: u8 = 3;
	pub const Time: u8 = 4;
	pub const Select: u8 = 5;
	pub const Plural: u8 = 6;
	pub const Pound: u8 = 7;
	pub const Tag: u8 = 8;
}

pub enum PluralType {
	Cardinal,
	Ordinal,
}

impl FromStr for PluralType {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"cardinal" => Ok(Self::Cardinal),
			"ordinal" => Ok(Self::Ordinal),
			other => anyhow::bail!("Invalid plural type: {other}"),
		}
	}
}

pub struct OptionValue {
	pub value: Vec<FormatJsNode>,
}

pub enum FormatJsNode {
	Literal(String),
	Argument(String),
	Number {
		value: String,
		style: Option<String>,
	},
	Date {
		value: String,
		style: Option<String>,
	},
	Time {
		value: String,
		style: Option<String>,
	},
	Select {
		value: String,
		options: HashMap<String, OptionValue>,
		/// not implemented in js
		offset: u64,
	},
	Plural {
		value: String,
		options: HashMap<String, OptionValue>,
		/// not implemented in js
		offset: u64,
		plural_type: PluralType,
	},
	Pound,
	Tag {
		value: String,
		children: Vec<Self>,
		control: Option<Vec<Self>>,
	},
}
pub mod hydrate {
	use std::collections::HashMap;

	use anyhow::{Context as _, Result, bail};
	use serde_json::Value;

	use crate::intl::ast::{FormatJsNode, NodeType as NT, OptionValue};

	// FIXME: merge with hydrate_plural below, handle with helper/macro?
	fn hydrate_select(mut v: Vec<Value>) -> Result<FormatJsNode> {
		if v.len() != 4 {
			bail!("Plural node must have exactly 5 elements, got {v:?}");
		}
		let offset = v
			.pop()
			.unwrap()
			.as_u64()
			.context("expected fourth element to be a u64")?;
		let Value::Object(options) = v.pop().unwrap() else {
			bail!("expected third element to be an object");
		};
		let Value::String(value) = v.pop().unwrap() else {
			bail!("expected second element to be a string");
		};
		let mut hydrated_options = HashMap::with_capacity(options.len());
		for (k, v) in options {
			let Value::Array(v) = v else {
				bail!("expected option value to be an array, got {v:?}");
			};
			let v = v
				.into_iter()
				.map(hydrate_single)
				.collect::<Result<Vec<_>>>()?;
			hydrated_options.insert(k, OptionValue { value: v });
		}
		Ok(FormatJsNode::Select {
			value,
			options: hydrated_options,
			offset,
		})
	}
	fn hydrate_plural(mut v: Vec<Value>) -> Result<FormatJsNode> {
		if v.len() != 5 {
			bail!("Plural node must have exactly 5 elements, got {v:?}");
		}
		let plural_type = v
			.pop()
			.unwrap()
			.as_str()
			.context("expected fifth element to be a string")?
			.parse()?;
		let offset = v
			.pop()
			.unwrap()
			.as_u64()
			.context("expected fourth element to be a u64")?;
		let Value::Object(options) = v.pop().unwrap() else {
			bail!("expected third element to be an object");
		};
		let Value::String(value) = v.pop().unwrap() else {
			bail!("expected second element to be a string");
		};
		let mut hydrated_options = HashMap::with_capacity(options.len());
		for (k, v) in options {
			let Value::Array(v) = v else {
				bail!("expected option value to be an array, got {v:?}");
			};
			let v = hydrate_array(v)?;
			hydrated_options.insert(k, OptionValue { value: v });
		}
		Ok(FormatJsNode::Plural {
			value,
			options: hydrated_options,
			offset,
			plural_type,
		})
	}

	fn hydrate_array(v: Vec<Value>) -> Result<Vec<FormatJsNode>> {
		v.into_iter()
			.map(hydrate_single)
			.collect()
	}

	fn hydrate_tag(mut v: Vec<Value>) -> Result<FormatJsNode> {
		let l = v.len();
		if l != 3 && l != 4 {
			bail!("Tag node must have 3 or 4 elements, got {v:?}");
		}
		let control = if l == 4 {
			match v.pop().unwrap() {
				Value::Array(v) => Some(v),
				Value::Null => None,
				other => bail!(
					"expected fourth element to be an array or null, got {other:?}"
				),
			}
		} else {
			None
		};
		let Value::Array(children) = v.pop().unwrap() else {
			bail!("expected third element to be an array, got {v:?}");
		};
		let Value::String(value) = v.pop().unwrap() else {
			bail!("expected second element to be a string, got {v:?}");
		};
		let hydrated_children = hydrate_array(children)?;
		let hydrated_control = control.map(hydrate_array).transpose()?;
		Ok(FormatJsNode::Tag {
			value,
			children: hydrated_children,
			control: hydrated_control,
		})
	}

	fn hydrate_single(v: Value) -> Result<FormatJsNode> {
		if let Value::String(s) = v {
			Ok(FormatJsNode::Literal(s))
		} else {
			let Value::Array(mut v) = v else {
				bail!("Expected array, got {v:?}");
			};
			let Some(Value::Number(n)) = v.first() else {
				bail!(
					"expected first element to be enum tag (number), got {:?}",
					v.first()
				);
			};
			let Some(tag) = n
				.as_u64()
				.and_then(|a| u8::try_from(a).ok())
			else {
				bail!("expected first element to be an u8, got {n:?}");
			};
			macro_rules! num_date_time {
				($var:ident) => {{
					if !matches!(v.len(), 2 | 3) {
						bail!(
							"{} node must have 2 or 3 elements, got {v:?}",
							stringify!($var)
						);
					}
					let style = if v.len() == 3 {
						let v = v.pop().unwrap();
						match v {
							Value::String(s) => Some(s),
							Value::Null => None,
							other => bail!(
								"expected third element to be a string or null, got {other:?}"
							),
						}
					} else {
						None
					};
					let Value::String(value) = v.pop().unwrap() else {
						bail!(
							"expected second element to be a string, got {v:?}"
						);
					};
					FormatJsNode::$var { value, style }
				}};
			}
			let ret = match tag {
				NT::Argument => {
					if v.len() != 2 {
						bail!(
							"Argument node must have exactly 2 elements, got {v:?}"
						);
					}
					let Value::String(s) = v.pop().unwrap() else {
						bail!("expected second element to be a string");
					};
					FormatJsNode::Argument(s)
				}
				NT::Number => {
					num_date_time!(Number)
				}
				NT::Date => {
					num_date_time!(Date)
				}
				NT::Time => {
					num_date_time!(Time)
				}
				NT::Select => hydrate_select(v)?,
				NT::Plural => hydrate_plural(v)?,
				NT::Pound => FormatJsNode::Pound,
				NT::Tag => hydrate_tag(v)?,
				_ => {
					bail!("Unsupported node type {tag}");
				}
			};
			Ok(ret)
		}
	}

	pub fn hydrate_ast(v: Vec<Value>) -> Result<Vec<FormatJsNode>> {
		if v.first().is_some_and(Value::is_string) {
			hydrate_array(v)
		} else if v.is_empty() {
			Ok(Vec::new())
		} else if v.first().is_none_or(|v| !v.is_array()) {
			hydrate_single(Value::Array(v)).map(|v| vec![v])
		} else {
			hydrate_array(v)
		}
	}
}
