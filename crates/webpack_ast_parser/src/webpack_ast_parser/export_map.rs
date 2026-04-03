use oxc::{ast::AstKind, span::Span};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMap<T> {
	#[serde(default, skip_serializing_if = "HashMap::is_empty")]
	pub exports: HashMap<String, ExportValue<T>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cjs_default: Option<Box<ExportValue<T>>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub hover: Option<String>,
}

impl<T> Default for ExportMap<T> {
	fn default() -> Self {
		Self {
			exports: Default::default(),
			cjs_default: Default::default(),
			hover: Default::default(),
		}
	}
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ExportValue<T> {
	Nodes(Vec<T>),
	Nested(ExportMap<T>),
}

pub type RawExportMap<'ast> = ExportMap<AstKind<'ast>>;

pub type RangeExportMap<'ast> = ExportMap<Span>;
