use ast_parser::{
	AstParser as _,
	ast_kind::IntoAstKind,
	exts::{
		BindingPatternExt as _,
		ExpressionExt as _,
		Functionish,
		StatementExt as _,
	},
};
use daft::Diffable;
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
use oxc::{
	ast::{
		AstKind,
		ast::{
			ArrowFunctionExpression,
			CallExpression,
			Class,
			ClassElement,
			Expression,
			IdentifierReference,
			LogicalOperator,
			MethodDefinition,
			MethodDefinitionKind,
			NewExpression,
			ObjectExpression,
			ObjectProperty,
			ObjectPropertyKind,
			SequenceExpression,
			VariableDeclarator,
		},
	},
	semantic::NodeId,
	span::{GetSpan as _, Span},
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::{
	cell::RefCell,
	collections::HashMap,
	convert::AsMut,
	fmt::Debug,
	iter,
};
use tracing::{debug, warn};

use crate::{
	WebpackAstParser,
	parser::{
		enum_iife::EnumIIFEState1_2,
		types::WreqDExportType,
		util::find_return_identifier,
	},
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Diffable)]
#[serde(rename_all = "camelCase")]
pub struct ExportMap<T> {
	#[serde(default, skip_serializing_if = "HashMap::is_empty")]
	pub exports: HashMap<SmolStr, ExportValue<T>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cjs_default: Option<Box<ExportValue<T>>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub hover: Option<SmolStr>,
	pub extra_data: ExtraData<T>,
	/// The node this whole map was made from, such as the object literal in
	/// `wreq.d(exports, { foo: () => obj }); const obj = { bar: 1 };`
	///
	/// Only set on raw export maps
	#[serde(skip)]
	#[daft(ignore)]
	pub node: Option<T>,
}

impl<T> ExportMap<T> {
	pub fn is_empty(&self) -> bool {
		self.exports.is_empty() && self.cjs_default.is_none()
	}
	/// Shallow merge of two export maps.
	/// [`self`] takes precedence over `other`
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
		// a merged map is not made from any single node
		self.node = if self.is_empty() && self.node.is_none() {
			other.node
		} else {
			None
		};
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

#[derive(
	Debug, Default, Clone, Serialize, Unwrap, IsVariant, PartialEq, Eq, Diffable,
)]
#[unwrap(ref, ref_mut)]
pub enum ExtraData<T> {
	#[default]
	None,
	Store(StoreData<T>),
}

/// Methods and props can be found in the attached [`ExportMap`]
/// the store name will be in [`ExportMap::hover`] if it can be found
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreData<T> {
	/// will always be a reference to the store identifier. an [`AstKind::BindingIdentifier`]
	pub store: T,
	/// map of flux events to their handlers
	pub flux_events: HashMap<SmolStr, T>,
}

impl<T> From<Option<StoreData<T>>> for ExtraData<T> {
	fn from(value: Option<StoreData<T>>) -> Self {
		value.map_or_else(|| Self::None, |store_data| Self::Store(store_data))
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
			node: None,
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

	// FIXME: husk
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

#[derive(
	Debug, Clone, PartialEq, Eq, Serialize, From, IsVariant, TryUnwrap,
)]
#[doc(alias("AnyExportKey"))]
#[try_unwrap(ref, ref_mut)]
/// Clone is `O(1)`
pub enum ExportMapKey {
	Named(SmolStr),
	Default,
}

impl<T: AsRef<str>> From<&T> for ExportMapKey {
	fn from(s: &T) -> Self {
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
			node: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deref, DerefMut, PartialEq, Eq, Diffable)]
pub struct ExportRange<T>(
	#[deref]
	#[deref_mut]
	#[daft(leaf)]
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
	#[must_use]
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

#[derive(
	Debug,
	Clone,
	Serialize,
	From,
	Unwrap,
	TryUnwrap,
	IsVariant,
	PartialEq,
	Eq,
	Diffable,
)]
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
	pub const fn get_hover(&self) -> Option<&SmolStr> {
		match self {
			Self::Range(ExportRange(_, hover))
			| Self::Map(ExportMap { hover, .. }) => hover.as_ref(),
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

thread_local! {
	pub static MAKE_MAP_STACK: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

fn get_ast_kind_span_for_export_map(node: AstKind) -> Span {
	match node {
		AstKind::Function(func) => {
			// this will only be called on javascript code; therefore, all functions have bodies
			let body_span = func.body.as_ref().unwrap().span();
			let full_span = func.span();
			debug_assert!(
				full_span.start <= body_span.start
					&& full_span.end >= body_span.end
			);
			Span::new(full_span.start, body_span.start)
		}
		AstKind::ArrowFunctionExpression(func) => {
			let body_span = func.body.span();
			let full_span = func.span();
			debug_assert!(
				full_span.start <= body_span.start
					&& full_span.end >= body_span.end
			);
			Span::new(full_span.start, body_span.start)
		}
		other => other.span(),
	}
}
fn raw_export_range_to_range_export_range(
	ExportRange(nodes, annotation): &RawExportRange,
) -> RangeExportRange {
	let mut ret = nodes
		.iter()
		.copied()
		.map(get_ast_kind_span_for_export_map)
		.collect::<RangeExportRange>();
	// smol_str is O(1) clone
	ret.1.clone_from(annotation);
	ret
}
// TODO: transform extra data?
pub(super) fn raw_export_map_to_range_export_map(
	ExportMap {
		exports,
		cjs_default,
		hover,
		extra_data: _,
		node: _,
	}: &RawExportMap,
) -> RangeExportMap {
	RangeExportMap {
		exports: exports
			.iter()
			.map(|(k, v)| {
				(k.clone(), raw_export_value_to_range_export_value(v))
			})
			.collect(),
		cjs_default: cjs_default
			.as_ref()
			.map(|e| Box::new(raw_export_value_to_range_export_value(e))),
		hover: hover.clone(),
		extra_data: ExtraData::None,
		node: None,
	}
}
fn raw_export_value_to_range_export_value(
	v: &RawExportMapValue,
) -> RangeExportMapValue {
	match v {
		ExportValue::Range(r) => {
			ExportValue::Range(raw_export_range_to_range_export_range(r))
		}
		ExportValue::Map(m) => {
			ExportValue::Map(raw_export_map_to_range_export_map(m))
		}
	}
}

/// functions to make the raw export map
impl<'ast> WebpackAstParser<'ast> {
	/// ### Style 1:
	/// ```js
	/// function (e) {
	///     return e.foo = "foo",
	///     e.bar = "bar",
	///     e
	/// }({})
	/// ```
	/// ### Style 2:
	/// ```js
	/// function (e) {
	///     return e[e.foo = 1] = "foo",
	///     e[e.bar = 2] = "bar",
	///     e
	/// }({})
	/// ```
	fn try_raw_make_export_map_for_enum_iife_style_1_and_2(
		&self,
		node: &'ast CallExpression<'ast>,
	) -> Option<RawExportMap<'ast>> {
		// `({})` in `function(e) {...}({})`
		let args = &node.arguments;
		// we are only ever called with one argument
		if args.len() != 1 {
			return None;
		}
		// `{}` in `function(e) {...}({})`, or the namespace-style
		// `r || {}` in `function(e) {...}(r || {})`
		let arg_obj = match args[0].as_expression()? {
			Expression::ObjectExpression(o) => o.as_ref(),
			Expression::LogicalExpression(l)
				if l.operator == LogicalOperator::Or =>
			{
				l.right.as_object_expression()?
			}
			_ => return None,
		};
		if !arg_obj.properties.is_empty() {
			return None;
		}
		// check body
		let func = node.callee.as_function_expression()?;
		let func_body = &func.body.as_ref()?.statements;
		if func_body.len() != 1 {
			return None;
		}
		let stmt = func_body[0]
			.as_return_statement()?
			.argument
			.as_ref()?
			.as_sequence_expression()?;
		// check parameters and get a handle to the final enum object parameter
		if func.params.items.len() != 1 || func.params.rest.is_some() {
			return None;
		}
		// FIXME: assert no random things on enum_param, (private, decorators, etc...)
		let enum_param = func.params.items[0]
			.pattern
			.as_binding_identifier()?;
		// the last `,e` in the return statement
		// TODO: Refactor this state/logic into a separate struct?
		let enum_exprs = &stmt.expressions[..stmt.expressions.len() - 1];
		// a sequence expression should never be empty; therefore, this unwrap is safe
		let last_expr = stmt
			.expressions
			.last()
			.unwrap()
			.as_identifier()?;
		if !self.cmp_sym(last_expr, enum_param) {
			return None;
		}

		let mut state = EnumIIFEState1_2 {
			p: self,
			enum_param: enum_param.symbol_id(),
			ret: RawExportMap::default(),
		};

		for expr in enum_exprs {
			let expr = expr.as_assignment_expression()?;
			state.process(expr)?;
		}

		let mut ret = state.ret;

		if let Some(decl) = self
			.p(node.node_id())
			.as_variable_declarator()
			&& let Some(name) = decl.id.as_binding_identifier()
		{
			// sanity, should be impossible to hit
			debug_assert!(ret.cjs_default.is_none());
			ret.cjs_default =
				Some(Box::new(RawExportRange::from_node(name).into()));
		} else {
			debug_assert!(
				false,
				"Enum IIFEs should always be a variable initializer"
			);
		}

		Some(ret)
	}
	fn try_raw_make_export_map_for_enum_iife(
		&self,
		node: &'ast CallExpression<'ast>,
	) -> Option<RawExportMap<'ast>> {
		self.try_raw_make_export_map_for_enum_iife_style_1_and_2(node)
	}
	/// Sequence-expression enum style. Instead of an IIFE, the enum object is
	/// built inline via a parenthesized sequence expression, seeding itself on
	/// the first entry:
	/// ```js
	/// var a = ((l = {}).FUZZY = "fuzzy",
	///     l.EXACT = "exact",
	///     l.REGEX = "regex",
	///     l);
	/// ```
	/// Reuses the style 1/2 entry logic from the IIFE parser.
	fn try_raw_make_export_map_for_enum_style_3(
		&self,
		node: &'ast SequenceExpression<'ast>,
	) -> Option<RawExportMap<'ast>> {
		let exprs = &node.expressions;
		// at minimum one entry + the trailing `,e`
		if exprs.len() < 2 {
			return None;
		}
		// the last `,e` is the finished enum object
		let last = exprs.last().unwrap().as_identifier()?;
		let enum_param = self.sym_id_of(last)?;

		let mut state = EnumIIFEState1_2 {
			p: self,
			enum_param,
			ret: RawExportMap::default(),
		};

		for expr in &exprs[..exprs.len() - 1] {
			let expr = expr.as_assignment_expression()?;
			state.process(expr)?;
		}

		let mut ret = state.ret;

		// walk past the wrapping parens to find the `var a = (...)` binding
		let mut parent = self.p(node.node_id());
		while let AstKind::ParenthesizedExpression(_) = parent {
			parent = self.p(parent.node_id());
		}
		if let Some(decl) = parent.as_variable_declarator()
			&& let Some(name) = decl.id.as_binding_identifier()
		{
			debug_assert!(ret.cjs_default.is_none());
			ret.cjs_default =
				Some(Box::new(RawExportRange::from_node(name).into()));
		}

		Some(ret)
	}
	fn raw_make_export_map_object_expression(
		&self,
		node: &'ast ObjectExpression<'ast>,
	) -> RawExportMap<'ast> {
		// TODO: we can probably remove this box dyn iter if we use a manual for loop
		let mut ret: RawExportMap<'ast> = node
			.properties
			.iter()
			.filter_map(
				|prop| -> Option<
					Box<
						dyn Iterator<Item = (SmolStr, RawExportMapValue<'ast>)>,
					>,
				> {
					match prop {
						ObjectPropertyKind::ObjectProperty(prop) => {
							let key_txt =
								SmolStr::new(&self.source[prop.key.span()]);
							let mut val = self
								.raw_make_export_map_property_assignment(prop);
							if let Some(def_arr) =
								val.try_unwrap_map_mut().ok().and_then(
									ExportMap::get_default_arr_mut_if_exists,
								) {
								def_arr.insert(0, prop.key.into_ast_kind());
							}
							Some(Box::new(iter::once((key_txt, val))))
						}
						ObjectPropertyKind::SpreadProperty(spread_val) => {
							let spread_val = spread_val
								.argument
								.get_inner_expression();
							if !spread_val.is_identifier_reference() {
								debug!(
									"Spread assignment is not an identifier, this should be handled"
								);
							}
							let spread =
								self.raw_make_export_map_recursive(spread_val);
							let Ok(spread) = spread.try_unwrap_map() else {
								debug!(
									"Identifier in object spread is not an object, this should be handled"
								);
								return None;
							};
							// discard annotation and default
							Some(Box::new(spread.exports.into_iter()))
						}
					}
				},
			)
			.flatten()
			.collect();
		ret.node = Some(node.into_ast_kind());
		ret
	}
	fn raw_make_export_map_literalish(
		&self,
		node: AstKind<'ast>,
	) -> RawExportRange<'ast> {
		let annotation = SmolStr::new(self.text(&node));
		RawExportRange::annotated(iter::once(node), annotation)
	}
	fn raw_make_export_map_property_assignment(
		&self,
		node: &'ast ObjectProperty<'ast>,
	) -> RawExportMapValue<'ast> {
		let obj_range = self.raw_make_export_map_recursive(&node.value);
		match obj_range {
			ExportValue::Range(mut export_range) => {
				// FIXME: this seems... wrong
				export_range.insert(0, node.key.into_ast_kind());
				export_range.into()
			}
			map @ ExportValue::Map(_) => map,
		}
	}
	fn raw_make_export_map_functionish(
		&self,
		node: Functionish<'ast, 'ast>,
	) -> RawExportMapValue<'ast> {
		// handle if this is just a wrapper function (eg: `() => local_foo`)
		'wrapper_func_check: {
			// arrow_expr is a identifier or member expression
			if let Some(arrow_expr) = node
				.as_arrow()
				.and_then(ArrowFunctionExpression::get_expression)
				.map(WreqDExportType::try_from)
				.transpose()
				.ok()
				.flatten()
			{
				let ret = self.raw_make_export_map_recursive(arrow_expr);
				if !ret.is_empty() {
					return ret;
				}
			}
			// function () {return;} || () => {return;} || () => void 0
			if node
				.body()
				.is_some_and(|b| b.statements.len() == 1)
				|| node
					.as_arrow()
					.is_some_and(ArrowFunctionExpression::is_expression)
			{
				let Some(ident) = find_return_identifier(node) else {
					break 'wrapper_func_check;
				};
				let ret = self.raw_make_export_map_recursive(ident);
				if !ret.is_empty() {
					return ret;
				}
			}
		};
		let node = node
			.id()
			.map_or_else(|| node.into_ast_kind(), IntoAstKind::into_ast_kind);
		RawExportRange::from(node).into()
	}
	fn raw_make_export_map_call_expression(
		&self,
		node: &'ast CallExpression<'ast>,
	) -> RawExportMapValue<'ast> {
		if let Some(enum_export) =
			self.try_raw_make_export_map_for_enum_iife(node)
		{
			return enum_export.into();
		}
		// `Object.freeze({...})` — descend into the wrapped object literal
		if let Some(frozen) = Self::unwrap_object_freeze(node) {
			let mut ret = self.raw_make_export_map_object_expression(frozen);
			// `var a = Object.freeze({...})` set cjs_default at `a`
			let mut parent = self.p(node.node_id());
			while let AstKind::ParenthesizedExpression(_) = parent {
				parent = self.p(parent.node_id());
			}
			if ret.cjs_default.is_none()
				&& let Some(decl) = parent.as_variable_declarator()
				&& let Some(name) = decl.id.as_binding_identifier()
			{
				ret.cjs_default =
					Some(Box::new(RawExportRange::from_node(name).into()));
			} else {
				warn!(
					"Failed to set cjs_default for Object.freeze, parent is not variable declarator"
				);
			}
			return ret.into();
		}
		RawExportRange::from_node(node).into()
	}
	/// If `node` is a call to `Object.freeze(objectLiteral)`, return the
	/// wrapped object literal.
	fn unwrap_object_freeze(
		node: &'ast CallExpression<'ast>,
	) -> Option<&'ast ObjectExpression<'ast>> {
		let Expression::StaticMemberExpression(m) = &node.callee else {
			return None;
		};
		if m.property.name != "freeze" {
			return None;
		}
		let obj = m.object.as_identifier()?;
		if obj.name != "Object" {
			return None;
		}
		if node.arguments.len() != 1 {
			return None;
		}
		node.arguments[0]
			.as_expression()?
			.get_inner_expression()
			.as_object_expression()
	}
	fn raw_make_export_map_ident_ref(
		&self,
		node: &'ast IdentifierReference<'ast>,
	) -> RawExportMapValue<'ast> {
		'a: {
			let Some(sym_id) = self.sym_id_of(node) else {
				break 'a;
			};
			let trail = self.unwrap_variable_declarator(sym_id);
			let last_sym_id = *trail.last().unwrap_or(&sym_id);
			let last_node_id = self
				.sema
				.scoping()
				.symbol_declaration(last_sym_id);
			let last_node = self.n(last_node_id);
			return self.raw_make_export_map_recursive(last_node);
		};
		RawExportRange::from_node(node).into()
	}
	fn raw_make_export_map_variable_declarator(
		&self,
		node: &'ast VariableDeclarator<'ast>,
	) -> RawExportMapValue<'ast> {
		node.init.as_ref().map_or_else(
			|| RawExportRange::from_node(&node.id).into(),
			|init| self.raw_make_export_map_recursive(init),
		)
	}
	fn raw_make_export_map_class(
		&self,
		node: &'ast Class<'ast>,
	) -> RawExportMap<'ast> {
		let mut ret = RawExportMap::default();
		if let Some(name) = &node.id {
			ret.cjs_default =
				Some(Box::new(RawExportRange::from_node(name).into()));
		} else {
			ret.cjs_default =
				Some(Box::new(RawExportRange::from_node(node).into()));
		}
		for member in &node.body.body {
			match member {
				ClassElement::MethodDefinition(node) => {
					if node.kind == MethodDefinitionKind::Constructor {
						let ctor = node.key.into_ast_kind();
						ret.cjs_default
							.as_mut()
							.unwrap() // asd
							.unwrap_range_mut()
							.push(ctor);
					} else {
						let key = node.key.span();
						let key_txt = SmolStr::new(&self.source[key]);

						let val = node.as_ref();
						let val = self.raw_make_export_map_recursive(val);

						ret.exports.insert(key_txt, val);
					}
				}
				ClassElement::PropertyDefinition(node) => {
					let key_txt = SmolStr::new(&self.source[node.key.span()]);
					let val = node.value.as_ref().map_or_else(
						|| node.key.into_ast_kind(),
						IntoAstKind::into_ast_kind,
					);
					let val = self.raw_make_export_map_recursive(val);
					ret.exports.insert(key_txt, val);
				}
				ClassElement::AccessorProperty(_) => {
					unimplemented!("handle accessor")
				}
				ClassElement::TSIndexSignature(_) => unreachable!(
					"TSIndexSignature should not be present in JS code"
				),
				ClassElement::StaticBlock(_) => {}
			}
		}

		ret
	}

	/// this is pretty much a copy of [`Self::raw_make_export_map_functionish`]
	fn raw_make_export_map_method_definition(
		&self,
		node: &'ast MethodDefinition<'ast>,
	) -> RawExportMapValue<'ast> {
		let func = node.value.as_ref();
		if func
			.body
			.as_ref()
			.unwrap()
			.statements
			.len()
			== 1
			&& let Some(ident) =
				find_return_identifier(Functionish::Named(func))
		{
			let ret = self.raw_make_export_map_recursive(ident);
			if !ret.is_empty() {
				return ret;
			}
		}
		RawExportRange::from_node(&node.key).into()
	}

	/// Try to make a raw export map for a discord store
	fn raw_make_export_map_store(
		&self,
		init: &'ast NewExpression<'ast>,
	) -> Option<RawExportMap<'ast>> {
		let init_expr = init.callee.as_identifier()?;
		let store_sym_id = self.sym_id_of(init_expr)?;
		if !matches!(init.arguments.len(), 0 | 2 | 3) {
			debug!("Maybe store does not have 0, 2 or 3 ctor args");
			return None;
		}
		let mut ret = RawExportMap {
			extra_data: ExtraData::Store(RawStoreData {
				store: init_expr.into_ast_kind(),
				flux_events: HashMap::new(),
			}),
			..Default::default()
		};
		if init.arguments.len() >= 2 {
			// (flux, {/*events obj */}) for Flux Stores
			// (flux, {/*events obj */}, priority) for Flux Stores
			// ({/*events obj */}, mode) for libdiscore stores
			let events_obj = init.arguments[1]
				.as_object_expression()
				.or_else(|| init.arguments[0].as_object_expression())?;
			self.parse_store_flux_events(
				ret.extra_data.unwrap_store_mut(),
				events_obj,
			);
		}
		// TODO: unwrap variable declarator?
		let store_decl_id = self
			.sema
			.scoping()
			.symbol_declaration(store_sym_id);
		let store_decl = self.n(store_decl_id).as_class()?;
		let does_extend = store_decl.heritage.is_some();
		if !does_extend {
			debug!("Maybe store does not extend any class.");
			return None;
		}
		ret.merge_with(self.raw_make_export_map_class(store_decl));
		if let Some(store_name) = ret.exports.get("displayName") {
			if let ExportValue::Range(ExportRange(_, name)) = store_name {
				debug_assert!(
					ret.hover.is_none(),
					"Store hover should not be set"
				);
				if name.is_none() {
					warn!(
						module_id=?self.get_module_id().ok(),
						"Store has displayName prop but could not resolve display name. This should not happen"
					);
				}
				ret.hover = name.as_deref().map(|name| {
					// NOTE: this is a cheap hack to remove quotes from the string
					// this assumes that displayName is always ascii and never has any escapes
					// we *could* use serde_json to parse name as a string in other cases
					SmolStr::from(
						name.trim_start_matches('"')
							.trim_end_matches('"'),
					)
				});
			} else {
				warn!(
					"Store displayName prop is not a range. This should not happen."
				);
			}
		} else if let Some(store_name) = self.try_find_store_name(store_sym_id)
		{
			debug_assert!(ret.hover.is_none(), "Store hover should not be set");
			ret.hover = Some(store_name);
		}
		// add the new expr to the cjs default chain
		ret.get_default_arr_mut()
			.insert(0, init_expr.into_ast_kind());
		Some(ret)
	}

	fn raw_make_export_map_new_expression(
		&self,
		node: &'ast NewExpression<'ast>,
	) -> RawExportMapValue<'ast> {
		if let Some(store) = self.raw_make_export_map_store(node) {
			return store.into();
		}
		RawExportRange::from_node(node).into()
	}

	pub(super) fn raw_make_export_map_recursive(
		&self,
		node: impl IntoAstKind<'ast>,
	) -> RawExportMapValue<'ast> {
		/// Prevent stack overflow
		struct StackGuard(NodeId);
		impl StackGuard {
			fn push(id: AstKind) -> Self {
				let id = id.node_id();
				MAKE_MAP_STACK.with_borrow_mut(|v| {
					if v.contains(&id) {
						panic!("recursive export map; would stack overflow. TODO: return sane-ish value instead of erroring");
					}
					v.push(id);
				});
				Self(id)
			}
		}
		impl Drop for StackGuard {
			fn drop(&mut self) {
				MAKE_MAP_STACK.with_borrow_mut(|v| {
					let v = v.pop();
					debug_assert!(
						v == Some(self.0),
						"export map stack overflow guard mismatch"
					);
				});
			}
		}
		let node = node.into_ast_kind();
		let _guard = StackGuard::push(node);
		match node {
			AstKind::ObjectExpression(node) => self
				.raw_make_export_map_object_expression(node)
				.into(),
			AstKind::TemplateLiteral(t) if t.is_no_substitution_template() => {
				self.raw_make_export_map_literalish(node)
					.into()
			}
			AstKind::BooleanLiteral(_)
			| AstKind::NullLiteral(_)
			| AstKind::NumericLiteral(_)
			| AstKind::StringLiteral(_)
			| AstKind::BigIntLiteral(_)
			| AstKind::RegExpLiteral(_) => self
				.raw_make_export_map_literalish(node)
				.into(),
			AstKind::ObjectProperty(node) => {
				self.raw_make_export_map_property_assignment(node)
			}
			AstKind::ArrowFunctionExpression(node) => {
				self.raw_make_export_map_functionish(Functionish::from(node))
			}
			AstKind::Function(node) => {
				self.raw_make_export_map_functionish(Functionish::from(node))
			}
			AstKind::CallExpression(node) => {
				self.raw_make_export_map_call_expression(node)
			}
			AstKind::IdentifierReference(node) => {
				self.raw_make_export_map_ident_ref(node)
			}
			// TODO: Not sure if this is correct or if this should be handled as a special case in raw_make_export_map_ident_ref
			AstKind::VariableDeclarator(node) => {
				self.raw_make_export_map_variable_declarator(node)
			}
			AstKind::Class(node) => self
				.raw_make_export_map_class(node)
				.into(),
			AstKind::MethodDefinition(node) => {
				self.raw_make_export_map_method_definition(node)
			}
			AstKind::NewExpression(node) => {
				self.raw_make_export_map_new_expression(node)
			}
			// `var a = ((l = {}).FOO = "foo", l.BAR = "bar", l)`
			AstKind::ParenthesizedExpression(paren) => paren
				.expression
				.as_sequence_expression()
				.and_then(|seq| {
					self.try_raw_make_export_map_for_enum_style_3(seq)
				})
				.map_or_else(|| RawExportRange::from(node).into(), Into::into),
			_ => {
				if cfg!(debug_assertions) && cfg!(test) {
					debug!(
						"Unhandled export map node kind: {}",
						node.debug_name()
					);
				}
				RawExportRange::from(node).into()
			}
		}
	}
}
