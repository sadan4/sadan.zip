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
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::{borrow::Borrow, collections::HashMap, convert::AsMut, fmt::Debug, iter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMap<T> {
	#[serde(default, skip_serializing_if = "HashMap::is_empty")]
	pub exports: HashMap<SmolStr, ExportValue<T>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cjs_default: Option<Box<ExportValue<T>>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub hover: Option<SmolStr>,
	pub extra_data: ExtraData<T>,
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
		match (&mut self.extra_data, other.extra_data) {
			(_, ExtraData::None) => {}
			(this @ ExtraData::None, other @ ExtraData::Store(_)) => {
				*this = other;
			}
			(ExtraData::Store(_), ExtraData::Store(_)) => {
				debug_assert!(
					false,
					"merging two export maps that both have store data is not supported"
				);
			}
		}
		self.exports.extend(other.exports);
		if self.cjs_default.is_none() {
			self.cjs_default = other.cjs_default;
		}
	}

	pub(crate) fn get_default_arr_mut_if_exists(
		&mut self,
	) -> Option<&mut ExportRange<T>> {
		match self
			.cjs_default
			.as_mut()
			.map(AsMut::as_mut)
		{
			Some(ExportValue::Map(map)) => map.get_default_arr_mut_if_exists(),
			Some(ExportValue::Range(range)) => Some(range),
			None => None,
		}
	}

	pub(crate) fn get_default_arr_mut(&mut self) -> &mut ExportRange<T> {
		match self
			.cjs_default
			.get_or_insert_with(|| {
				Box::new(ExportValue::Range(ExportRange::default()))
			})
			.as_mut()
		{
			ExportValue::Range(export_range) => export_range,
			ExportValue::Map(export_map) => export_map.get_default_arr_mut(),
		}
	}
	pub fn get(&self, key: &ExportMapKey) -> Option<&ExportValue<T>> {
		match key {
			ExportMapKey::Named(smol_str) => self.exports.get(smol_str),
			ExportMapKey::Default => self.cjs_default.as_deref(),
		}
	}
}

#[derive(Debug, Default, Clone, Serialize, Unwrap, IsVariant)]
#[unwrap(ref, ref_mut)]
pub enum ExtraData<T> {
	#[default]
	None,
	Store(StoreData<T>),
}

/// Methods and props can be found in the attached [`ExportMap`]
/// the store name will be in [`ExportMap::hover`] if it can be found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreData<T> {
	/// will always be a reference to the store identifier. an [`AstKind::BindingIdentifier`]
	pub store: T,
	/// map of flux events to their handlers
	pub flux_events: HashMap<SmolStr, T>,
}

impl<T> From<Option<StoreData<T>>> for ExtraData<T> {
	fn from(value: Option<StoreData<T>>) -> Self {
		match value {
			Some(store_data) => Self::Store(store_data),
			None => Self::None,
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
			extra_data: ExtraData::default(),
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

impl IntoIterator for RangeExportMap {
	type Item = RangeExportMapEntry;

	type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

	fn into_iter(self) -> Self::IntoIter {
		let def: Box<dyn Iterator<Item = Self::Item>> = if let Some(def) =
			self.cjs_default
		{
			Box::new(iter::once(ExportMapEntry(ExportMapKey::Default, *def)))
		} else {
			Box::new(iter::empty())
		};
		Box::new(
			self.exports
				.into_iter()
				.map(Into::into)
				.chain(def),
		)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, From, IsVariant)]
/// Clone is `O(1)`
pub enum ExportMapKey {
	Named(SmolStr),
	Default,
}

impl ExportMapKey {
	pub fn from_str(s: &impl AsRef<str>) -> Self {
		Self::Named(s.as_ref().into())
	}
}

impl<T> Default for ExportMap<T> {
	fn default() -> Self {
		Self {
			exports: HashMap::default(),
			cjs_default: None,
			hover: None,
			extra_data: ExtraData::default(),
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

impl<T> Default for ExportRange<T> {
	fn default() -> Self {
		Self(Vec::new(), None)
	}
}

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
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
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
	pub fn prepend_with(&mut self, val: T) {
		match self {
			Self::Range(rng) => rng.insert(0, val),
			Self::Map(map) => map.get_default_arr_mut().insert(0, val),
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
// pub type RawExportMapEntry<'ast> = ExportMapEntry<AstKind<'ast>>;
pub type RawExportRange<'ast> = ExportRange<AstKind<'ast>>;
pub type RawExportMap<'ast> = ExportMap<AstKind<'ast>>;
pub type RawStoreData<'ast> = StoreData<AstKind<'ast>>;

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
