use ast_parser::ast_kind::IntoAstKind;
use derive_more::{
	Constructor,
	Deref,
	DerefMut,
	From,
	Into,
	IsVariant,
	TryUnwrap,
	Unwrap,
};
use oxc::{ast::AstKind, span::Span};
use serde::Serialize;
use smol_str::SmolStr;
use std::{collections::HashMap, iter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMap<T> {
	#[serde(default, skip_serializing_if = "HashMap::is_empty")]
	pub exports: HashMap<SmolStr, ExportValue<T>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cjs_default: Option<Box<ExportValue<T>>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub hover: Option<SmolStr>,
}

impl<T> ExportMap<T> {
	pub fn is_empty(&self) -> bool {
		self.exports.is_empty() && self.cjs_default.is_none()
	}
	/// Shallow merge of two export maps.
	/// [`self`] takes precedence over [`other`]
	pub fn merge_with(&mut self, other: Self) {
		debug_assert!(
			!(self.cjs_default.is_some() && other.cjs_default.is_some()),
			"cannot merge two export maps that both have a default export"
		);
		self.exports.extend(other.exports);
		if self.cjs_default.is_none() {
			self.cjs_default = other.cjs_default;
		}
	}
}

impl<T> FromIterator<(SmolStr, ExportValue<T>)> for ExportMap<T> {
	fn from_iter<I: IntoIterator<Item = (SmolStr, ExportValue<T>)>>(
		iter: I,
	) -> Self {
		Self {
			exports: iter.into_iter().collect(),
			cjs_default: None,
			hover: None,
		}
	}
}

impl<T> FromIterator<(ExportMapKey, ExportValue<T>)> for ExportMap<T> {
	fn from_iter<I: IntoIterator<Item = (ExportMapKey, ExportValue<T>)>>(
		iter: I,
	) -> Self {
		let iter = iter.into_iter();
		let mut ret = Self::default();
		ret.exports.reserve(iter.size_hint().0);
		iter.fold(ret, |mut acc, (k, v)| {
			match k {
				ExportMapKey::Named(n) => {
					acc.exports.insert(n, v);
				}
				ExportMapKey::Default => {
					debug_assert!(
						acc.cjs_default.is_none(),
						"setting default export more than once"
					);
					acc.cjs_default = Some(Box::new(v));
				}
			}
			acc
		})
	}
}

#[derive(Debug, Clone, Serialize, From)]
pub enum ExportMapKey {
	Named(SmolStr),
	Default,
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

#[derive(Debug, Clone, Serialize, Deref, DerefMut)]
pub struct ExportRange<T>(
	#[deref]
	#[deref_mut]
	pub Vec<T>,
	pub Option<SmolStr>,
);

impl<T> From<T> for ExportRange<T> {
	fn from(value: T) -> Self {
		Self(vec![value], None)
	}
}

impl<T> ExportRange<T> {
	pub fn annotated(
		nodes: impl IntoIterator<Item = T>,
		hover: SmolStr,
	) -> Self {
		Self(Vec::from_iter(nodes), Some(hover))
	}
	pub fn annotate(&mut self, hover: SmolStr) {
		self.1 = Some(hover);
	}
	pub fn with_annotation(mut self, hover: SmolStr) -> Self {
		self.annotate(hover);
		self
	}
	pub const fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl<T> FromIterator<T> for ExportRange<T> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		Self(iter.into_iter().collect(), None)
	}
}

#[derive(Debug, Clone, Serialize, From, Unwrap, TryUnwrap, IsVariant)]
#[serde(untagged)]
pub enum ExportValue<T> {
	Range(ExportRange<T>),
	Map(ExportMap<T>),
}

impl<T> ExportValue<T> {
	/// Returns true if the export value is empty (ie: no exports)
	/// See: [`ExportMap::is_empty`] and [`ExportRange::is_empty`]
	pub fn is_empty(&self) -> bool {
		match self {
			Self::Range(r) => r.is_empty(),
			Self::Map(m) => m.is_empty(),
		}
	}
}

#[derive(Debug, Clone, Into, Constructor)]
pub struct ExportMapEntry<T>(pub ExportMapKey, pub ExportValue<T>);

impl<A, B> From<(A, ExportValue<B>)> for ExportMapEntry<B>
where
	A: Into<ExportMapKey>,
{
	fn from((k, v): (A, ExportValue<B>)) -> Self {
		Self(k.into(), v)
	}
}

pub type RawExportMapValue<'ast> = ExportValue<AstKind<'ast>>;
pub type RawExportMapEntry<'ast> = ExportMapEntry<AstKind<'ast>>;
pub type RawExportRange<'ast> = ExportRange<AstKind<'ast>>;
pub type RawExportMap<'ast> = ExportMap<AstKind<'ast>>;

pub type RangeExportMapValue = ExportValue<Span>;
pub type RangeExportMapEntry = ExportMapEntry<Span>;
pub type RangeExportRange = ExportRange<Span>;
pub type RangeExportMap = ExportMap<Span>;

impl<'ast> RawExportRange<'ast> {
	pub fn from_node(node: impl IntoAstKind<'ast>) -> Self {
		let node = node.into_ast_kind();
		Self::from(node)
	}
}
