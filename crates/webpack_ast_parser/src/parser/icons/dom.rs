use std::{borrow::Cow, collections::HashMap};

use oxc::span::Span;
use smol_str::SmolStr;

use crate::parser::value_tracking::{ConstantValue, SpannedValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
	pub tag: SmolStr,
	pub attrs: HashMap<SmolStr, AttrValue>,
	pub children: Vec<Child>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Child {
	Element(Node),
	/// A string or number child
	Text(SpannedValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
	Constant(SpannedValue),
	/// A color the icon component takes as a prop, eg
	/// `fill: "string" == typeof c ? c : c.css`
	ColorParam {
		span: Span,
		/// The prop's default when it is a string, eg `"transparent"`
		///
		/// `None` when the default is a design token or there is no
		/// default, meaning this is the icon's main color
		/// (`currentColor`)
		default: Option<SpannedValue>,
	},
}

/// Attributes React and SVG both spell in camelCase, every other camelCase
/// prop is written kebab-case (`fillRule` -> `fill-rule`)
const CAMEL_ATTRS: &[&str] = &[
	"viewBox",
	"preserveAspectRatio",
	"gradientUnits",
	"gradientTransform",
	"clipPathUnits",
	"maskUnits",
	"maskContentUnits",
	"patternUnits",
	"patternContentUnits",
	"patternTransform",
	"spreadMethod",
	"stdDeviation",
	"filterUnits",
	"primitiveUnits",
	"markerUnits",
	"markerWidth",
	"markerHeight",
	"refX",
	"refY",
	"startOffset",
	"textLength",
	"baseFrequency",
	"numOctaves",
	"edgeMode",
	"kernelMatrix",
	"tableValues",
	"calcMode",
	"keyTimes",
	"keySplines",
	"repeatCount",
	"attributeName",
];

impl Node {
	/// Renders this node as markup the way react-dom would
	///
	/// Attributes are sorted by name so the output is stable, color params
	/// are written as their default, or `currentColor` if they have none
	#[must_use]
	pub fn to_html(&self) -> String {
		let mut out = String::new();
		self.write_html(&mut out);
		out
	}

	fn write_html(&self, out: &mut String) {
		out.push('<');
		out.push_str(&self.tag);
		let mut attrs = self.attrs.iter().collect::<Vec<_>>();
		attrs.sort_unstable_by_key(|(name, _)| *name);
		for (name, value) in attrs {
			let value = match value {
				AttrValue::Constant(v) => {
					// react drops these instead of stringifying them
					if matches!(
						v.value,
						ConstantValue::Undefined
							| ConstantValue::Null
							| ConstantValue::Boolean(false)
					) {
						continue;
					}
					to_string(v)
				}
				AttrValue::ColorParam {
					default: Some(v), ..
				} => to_string(v),
				AttrValue::ColorParam { default: None, .. } => {
					Cow::Borrowed("currentColor")
				}
			};
			out.push(' ');
			out.push_str(&attr_name(name));
			out.push_str("=\"");
			escape(&value, true, out);
			out.push('"');
		}
		if self.children.is_empty() {
			out.push_str("/>");
			return;
		}
		out.push('>');
		for child in &self.children {
			match child {
				Child::Element(node) => node.write_html(out),
				Child::Text(v) => {
					// react renders nothing for these
					if !matches!(
						v.value,
						ConstantValue::Undefined
							| ConstantValue::Null
							| ConstantValue::Boolean(_)
					) {
						escape(&to_string(v), false, out);
					}
				}
			}
		}
		out.push_str("</");
		out.push_str(&self.tag);
		out.push('>');
	}
}

fn attr_name(react: &str) -> Cow<'_, str> {
	match react {
		"className" => return Cow::Borrowed("class"),
		"htmlFor" => return Cow::Borrowed("for"),
		"xlinkHref" => return Cow::Borrowed("xlink:href"),
		"xmlnsXlink" => return Cow::Borrowed("xmlns:xlink"),
		_ => {}
	}
	if CAMEL_ATTRS.contains(&react)
		|| !react.contains(|c: char| c.is_ascii_uppercase())
	{
		return Cow::Borrowed(react);
	}
	let mut out = String::with_capacity(react.len() + 2);
	for c in react.chars() {
		if c.is_ascii_uppercase() {
			out.push('-');
			out.push(c.to_ascii_lowercase());
		} else {
			out.push(c);
		}
	}
	Cow::Owned(out)
}

/// <https://262.ecma-international.org/#sec-tostring>
fn to_string(v: &SpannedValue) -> Cow<'_, str> {
	match &v.value {
		ConstantValue::String(s) => Cow::Borrowed(s),
		ConstantValue::Number(n) => Cow::Owned(n.to_string()),
		ConstantValue::NaN => Cow::Borrowed("NaN"),
		ConstantValue::BigInt(n) => Cow::Owned(n.to_string()),
		ConstantValue::Boolean(b) => {
			Cow::Borrowed(if *b { "true" } else { "false" })
		}
		ConstantValue::Undefined => Cow::Borrowed("undefined"),
		ConstantValue::Null => Cow::Borrowed("null"),
		ConstantValue::Array(elts) => Cow::Owned(
			elts.iter()
				.map(|elt| match elt.value {
					ConstantValue::Undefined | ConstantValue::Null => {
						Cow::Borrowed("")
					}
					_ => to_string(elt),
				})
				.collect::<Vec<_>>()
				.join(","),
		),
		ConstantValue::Object(_) => Cow::Borrowed("[object Object]"),
		ConstantValue::Set(_) => Cow::Borrowed("[object Set]"),
	}
}

fn escape(s: &str, attr: bool, out: &mut String) {
	for c in s.chars() {
		match c {
			'&' => out.push_str("&amp;"),
			'<' => out.push_str("&lt;"),
			'>' if !attr => out.push_str("&gt;"),
			'"' if attr => out.push_str("&quot;"),
			c => out.push(c),
		}
	}
}
