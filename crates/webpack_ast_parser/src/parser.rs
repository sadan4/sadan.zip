mod arg_finder;
mod enum_iife;
mod export_map;
mod types;
mod util;

use std::{iter, ops::Not};

use anyhow::Result;
use ast_parser::{
	AstParser,
	ast_kind::IntoAstKind,
	exts::{
		BindingPatternExt,
		ExpressionExt,
		Functionish,
		NumericLiteralExt as _,
		StatementExt,
	},
	parse,
	sym_id::GetSymId,
};
use itertools::Itertools;
use oxc::{
	allocator::Allocator,
	ast::{
		AstKind,
		ast::{
			ArrowFunctionExpression,
			AssignmentExpression,
			BindingIdentifier,
			CallExpression,
			ComputedMemberExpression,
			Expression,
			IdentifierReference,
			MemberExpression,
			NumericLiteral,
			ObjectExpression,
			ObjectProperty,
			ObjectPropertyKind,
			Program,
			StaticMemberExpression,
			VariableDeclarator,
		},
	},
	semantic::{NodeId, Semantic, SymbolId},
	span::{GetSpan, SourceType, Span},
};
use smol_str::SmolStr;
use tracing::{debug, trace};

use crate::{
	bundle::{DefaultModuleCache, IModuleCache},
	cache::{CacheRef, CacheValue},
	parser::{
		self,
		enum_iife::EnumIIFEState1_2,
		export_map::{
			ExportMap,
			ExportRange,
			ExportValue,
			RangeExportMap,
			RangeExportMapValue,
			RangeExportRange,
			RawExportMapEntry,
			RawExportMapValue,
			RawExportRange,
		},
		types::{WreqD, WreqDExportType},
		util::{find_return_identifier, find_return_member_expression},
	},
	types::ModuleId,
};

use export_map::RawExportMap;

pub struct WebpackAstParser<'ast> {
	prog: &'ast Program<'ast>,
	sema: Semantic<'ast>,
	source: &'ast str,
	module_cache: &'ast dyn IModuleCache<'ast>,
	/// Internal cache
	c: Cache<'ast>,
}

#[derive(Default)]
struct Cache<'ast> {
	wreq: CacheValue<Option<SymbolId>>,
	t: CacheValue<Option<SymbolId>>,
	raw_export_map: CacheRef<RawExportMap<'ast>>,
	range_export_map: CacheRef<RangeExportMap>,
	wreq_d: CacheValue<Option<WreqD<'ast>>>,
}

impl<'ast> AstParser<'ast> for WebpackAstParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}
}

impl<'ast> WebpackAstParser<'ast> {
	pub fn try_new(alloc: &'ast Allocator, source: &'ast str) -> Result<Self> {
		let (prog, sema) = parse(alloc, source, SourceType::script())?;
		Ok(Self {
			prog,
			sema,
			source,
			module_cache: &DefaultModuleCache,
			c: Cache::default(),
		})
	}

	#[must_use]
	pub fn with_module_cache(
		mut self,
		module_cache: &'ast dyn IModuleCache<'ast>,
	) -> Self {
		self.module_cache = module_cache;
		self
	}

	pub fn get_module_id(&self) -> Option<ModuleId> {
		const WEBPACK_MODULE_HEADER: &str = "// Webpack Module ";
		if self
			.source
			.starts_with(WEBPACK_MODULE_HEADER)
		{
			// `// Webpack Module 123456` -> parse the 123456
			let start = WEBPACK_MODULE_HEADER.len();
			let mut end = start;

			while end < self.source.len()
				&& self
					.source
					.chars()
					.nth(end)
					.unwrap()
					.is_ascii_digit()
			{
				end += 1;
			}

			if start == end {
				return None;
			}

			debug_assert!(
				self.source[start..end]
					.chars()
					.all(|c| c.is_ascii_digit())
			);
			let id = self.source[start..end].parse().ok()?;

			return Some(ModuleId(id));
		}
		None
	}
}

// Private API
#[allow(clippy::multiple_inherent_impl)]
impl<'ast> WebpackAstParser<'ast> {
	/// `exports` in `function(module, exports, wreq) {...}`
	/// also commonly `t` in `function(e, t, n) {...}`
	fn webpack_exports(&self) -> Option<SymbolId> {
		self.c
			.t
			.get(|| self.find_webpack_arg(1))
	}
	/// `__webpack_require__` in `function(module, exports, __webpack_require__) {...}`
	/// also commonly `t` in `function(e, t, n) {...}`
	fn wreq(&self) -> Option<SymbolId> {
		self.c
			.wreq
			.get(|| self.find_webpack_arg(2))
	}
	// TODO: would it be better to cache these in a vec or smth
	fn wreq_uses(&self) -> Option<impl Iterator<Item = AstKind<'ast>> + '_> {
		Some(self.ref_nodes(self.wreq()?))
	}
	fn text(&self, span: &impl GetSpan) -> &'ast str {
		&self.source[span.span()]
	}
	/// [`arg_index`]: the index of the param (0, 1, 2, ...)
	///
	/// Returns Some(SymbolId) of the param if found, or None if not found.
	///
	/// You should probably avoid this and use the other dedicated methods like [`Self::wreq`]
	/// which provide things like caching
	fn find_webpack_arg(&self, arg_index: u8) -> Option<SymbolId> {
		use arg_finder::find;
		find(self, arg_index)
	}

	// TODO: Add tests
	fn get_imported_var(&self, module_id: ModuleId) -> Option<SymbolId> {
		let usage = self.refs(self.wreq()?).find(|u| {
			self.find_parent(*u, AstKind::as_call_expression)
				.is_some_and(|call| {
					call.arguments.len() == 1
						&& call.arguments[0]
							.as_numeric_literal()
							.and_then(NumericLiteral::as_u32)
							.is_some_and(|n| n == *module_id)
				})
		})?;

		let ret = self
			.find_parent(usage, AstKind::as_variable_declarator)?
			.id
			.as_binding_identifier()?
			.symbol_id();

		Some(ret)
	}
	// FIXME: implement
	fn get_export_map_raw_wreq_e(&self) -> Option<RawExportMap<'ast>> {
		None
	}
	// FIXME: implement
	fn get_export_map_raw_wreq_t(&self) -> Option<RawExportMap<'ast>> {
		None
	}
	fn get_export_map_raw_wreq_d(&self) -> Option<RawExportMap<'ast>> {
		let exports_obj = self.find_wreq_d()?.obj;
		Some(self.raw_make_export_map_object_expression(exports_obj))
		// let ret = exports_obj
		// 	.properties
		// 	.iter()
		// 	.filter_map(|prop| -> Option<(SmolStr, RawExportMapValue<'ast>)> {
		// 		let prop = prop.as_property()?;
		// 		let val = prop.value.as_functionish()?;
		// 		let trailing_ident: WreqDExportType<'ast> =
		// 			find_return_identifier(val)
		// 				.map(Into::into)
		// 				.or_else(|| {
		// 					find_return_member_expression(val).map(Into::into)
		// 				})?;
		// 		// TODO: Support parsing stores here
		// 		// let ret: Option<RawExportMapValue<'ast>> = None;
		// 		// if ret.is_none()
		// 		// 	&& let Some(ident_sym_id) = trailing_ident
		// 		// 		.as_ident()
		// 		// 		.and_then(|i| i.get_sym_id(&self.sema))
		// 		// {
		// 		// 	ret = self
		// 		// 		.try_parse_class_decl(
		// 		// 			ident_sym_id,
		// 		// 			[prop.key.into_ast_kind()],
		// 		// 		)
		// 		// 		.map(Into::into);
		// 		// }
		// 		// let ret = ret.unwrap_or_else(|| {
		// 		// 	self.raw_make_export_map_recursive(trailing_ident)
		// 		// });
		// 		let ret = self.raw_make_export_map_recursive(trailing_ident);
		// 		let key_txt = SmolStr::new(&self.source[prop.key.span()]);
		//
		// 		Some((key_txt, ret))
		// 	})
		// 	.collect();
		// Some(ret)
	}

	// fn try_parse_class_decl<const N: usize>(
	// 	&self,
	// 	sym_id: SymbolId,
	// 	prefix: [AstKind<'ast>; N],
	// ) -> Option<RawExportMap<'ast>> {
	// 	todo!()
	// }

	fn impl_find_wreq_d(&self) -> Option<WreqD<'ast>> {
		// `t` in function(e, t, n) {...} where `n` is `__webpack_require__`
		let exports_decl = self.webpack_exports()?;
		for use_ in self.wreq_uses()? {
			// `wreq.d` in `wreq.d(...)`
			let Some(wreq_d_expr) = self
				.p(use_.node_id())
				.as_static_member_expression()
			else {
				continue;
			};
			// `d` in `wreq.d(...)`
			if wreq_d_expr.property.name != "d" {
				continue;
			}
			// `wreq.d(...)`
			let Some(call) = self
				.p(wreq_d_expr.node_id())
				.as_call_expression()
			else {
				continue;
			};
			// we should only ever have two arguments
			let args = &call.arguments;
			if args.len() != 2 {
				continue;
			}

			// `t` in `wreq.d(t, {...})`
			let Some(exports) = args[0].as_identifier() else {
				continue;
			};
			// ensure it's the exports
			// FIXME: don't think this could ever be `module.exports` instead of just `exports`
			// because wreq.d is only used on es modules
			if !self.cmp_sym(exports, &exports_decl) {
				continue;
			}
			// `{...}` in `wreq.d(t, {...})`
			let Some(obj) = args[1].as_object_expression() else {
				continue;
			};
			return Some(WreqD { call, exports, obj });
		}
		None
	}
	fn find_wreq_d(&self) -> Option<WreqD<'ast>> {
		self.c
			.wreq_d
			.get(|| self.impl_find_wreq_d())
	}
	fn impl_get_export_map_raw(&self) -> Option<RawExportMap<'ast>> {
		let mut ret: Option<RawExportMap<'ast>> = None;
		let mut merge = |new: Option<RawExportMap<'ast>>| match (&mut ret, new)
		{
			(None, new) => ret = new,
			(Some(ret), Some(new)) => ret.merge_with(new),
			_ => {}
		};
		merge(self.get_export_map_raw_wreq_d());
		merge(self.get_export_map_raw_wreq_t());
		merge(self.get_export_map_raw_wreq_e());
		ret
	}
	fn get_export_map_raw(&self) -> &RawExportMap<'ast> {
		self.c
			.raw_export_map
			.get_or_default(|| self.impl_get_export_map_raw())
	}
	fn get_export_map(&self) -> &RangeExportMap {
		self.c.range_export_map.get(|| {
			let raw = self.get_export_map_raw();
			Self::raw_export_map_to_range_export_map(raw)
		})
	}
}

// functions to make the raw export map
#[allow(clippy::multiple_inherent_impl)]
impl<'ast> WebpackAstParser<'ast> {
	fn get_ast_kind_span_for_export_map(node: AstKind<'ast>) -> Span {
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
				let body_span = func.body.as_ref().span();
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
		ExportRange(nodes, annotation): &RawExportRange<'ast>,
	) -> RangeExportRange {
		let mut ret = nodes
			.iter()
			.copied()
			.map(Self::get_ast_kind_span_for_export_map)
			.collect::<RangeExportRange>();
		// smol_str is O(1) clone
		ret.1.clone_from(annotation);
		ret.into()
	}
	fn raw_export_map_to_range_export_map(
		ExportMap {
			exports,
			cjs_default,
			hover,
		}: &RawExportMap<'ast>,
	) -> RangeExportMap {
		RangeExportMap {
			exports: exports
				.iter()
				.map(|(k, v)| {
					(k.clone(), Self::raw_export_value_to_range_export_value(v))
				})
				.collect(),
			cjs_default: cjs_default.as_ref().map(|e| {
				Box::new(Self::raw_export_value_to_range_export_value(e))
			}),
			hover: hover.clone(),
		}
	}
	fn raw_export_value_to_range_export_value(
		v: &RawExportMapValue<'ast>,
	) -> RangeExportMapValue {
		match v {
			ExportValue::Range(r) => ExportValue::Range(
				Self::raw_export_range_to_range_export_range(r),
			),
			ExportValue::Map(m) => {
				ExportValue::Map(Self::raw_export_map_to_range_export_map(m))
			}
		}
	}
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
		// `{}` in `function(e) {...}({})`
		if !args[0]
			.as_object_expression()?
			.properties
			.is_empty()
		{
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
			enum_param,
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
	fn raw_make_export_map_object_expression(
		&self,
		node: &'ast ObjectExpression<'ast>,
	) -> RawExportMap<'ast> {
		node.properties
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
							let val = self
								.raw_make_export_map_property_assignment(prop);
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
			.collect()
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
			if node.body().statements.len() == 1 {
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
			self.try_raw_make_export_map_for_enum_iife_style_1_and_2(node)
		{
			return enum_export.into();
		}
		return RawExportRange::from_node(node).into();
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
			let last_node = *self.n(last_node_id);
			return self.raw_make_export_map_recursive(last_node);
		};
		RawExportRange::from_node(node).into()
	}

	fn raw_make_export_map_variable_declarator(
		&self,
		node: &'ast VariableDeclarator<'ast>,
	) -> RawExportMapValue<'ast> {
		if let Some(init) = &node.init {
			self.raw_make_export_map_recursive(init)
		} else {
			// uninit variable
			RawExportRange::from_node(&node.id).into()
		}
	}

	fn raw_make_export_map_recursive(
		&self,
		node: impl IntoAstKind<'ast>,
	) -> RawExportMapValue<'ast> {
		let node = node.into_ast_kind();
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
			_ => RawExportRange::from(node).into(),
		}
	}
}

#[cfg(test)]
#[allow(clippy::unreadable_literal, clippy::too_many_lines)]
mod tests {
	use std::fmt::{self, Debug};

	use super::*;
	use ast_parser::span_line_and_column;
	use derive_more::Deref;
	use insta::{assert_debug_snapshot, assert_ron_snapshot};
	use itertools::Itertools;
	use oxc::span::{Atom, GetSpan as _, Span};

	macro_rules! parse {
		($alloc:expr, $source:literal) => {{
			let source = include_str!($source);
			WebpackAstParser::try_new(&$alloc, source).unwrap()
		}};
	}

	struct ExportMapDumper<'a>(pub &'a RangeExportMap, pub &'a str);

	impl ExportMapDumper<'_> {
		fn handle_value(
			&self,
			f: &mut fmt::Formatter<'_>,
			v: &RangeExportMapValue,
		) -> Result<(), fmt::Error> {
			match v {
				ExportValue::Range(range) => {
					let do_dbg_list = fmt::from_fn(|f| {
						let mut dbg_list = f.debug_list();
						for &span in range.iter() {
							let ((l1, c1), (l2, c2)) =
								span_line_and_column(self.1, span);
							dbg_list.entry(&format!("[{l1}:{c1}->{l2}:{c2})"));
						}
						dbg_list.finish()
					});
					if let Some(hover) = &range.1 {
						f.debug_tuple(hover.as_str())
							.field(&do_dbg_list)
							.finish()
					} else {
						do_dbg_list.fmt(f)
					}
				}
				ExportValue::Map(m) => {
					let dumper = ExportMapDumper(m, self.1);
					f.debug_tuple("ExportMap")
						.field(&dumper)
						.finish()
				}
			}
		}
	}

	impl Debug for ExportMapDumper<'_> {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			let mut dbg_map = f.debug_map();
			for (k, v) in self
				.0
				.exports
				.iter()
				.sorted_by(|a, b| a.0.cmp(b.0))
			{
				dbg_map.entry(&k, &fmt::from_fn(|f| self.handle_value(f, v)));
			}
			if let Some(v) = &self.0.cjs_default {
				let v = v.as_ref();
				dbg_map.entry(
					&"SYM_CJS_DEFAULT",
					&fmt::from_fn(|f| self.handle_value(f, v)),
				);
			}

			dbg_map.finish()
		}
	}

	impl<'ast> WebpackAstParser<'ast> {
		fn t_sym_info<'a>(&'a self, sym_id: SymbolId) -> (Atom<'a>, Span)
		where
			'ast: 'a,
		{
			let name = self
				.sema
				.scoping()
				.symbol_ident(sym_id)
				.as_atom();
			let span = self
				.sema
				.scoping()
				.symbol_declaration(sym_id);
			let node = self.n(span);
			(name, node.span())
		}
		fn dbg_export_map(&self) -> ExportMapDumper<'_> {
			ExportMapDumper(self.get_export_map(), self.source)
		}
	}

	#[test]
	fn constructs() {
		let alloc = Allocator::new();
		let source = include_str!("test_data/wp/module.js");
		_ = WebpackAstParser::try_new(&alloc, source).unwrap();
	}

	#[test]
	fn finds_wreq() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/module.js");
		let wreq = p.wreq().unwrap();
		let info = p.t_sym_info(wreq);
		assert_debug_snapshot!(info, @r#"
		(
		    "n",
		    Span {
		        start: 56,
		        end: 57,
		    },
		)
		"#);
	}

	#[test]
	fn doesnt_find_wreq_in_module_that_doesnt_use_it() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/bad/noWreq.js");
		assert_eq!(p.wreq(), None);
	}

	#[test]
	fn finds_imported_var() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/module.js");
		let info = p
			.get_imported_var(200651.into())
			.unwrap();
		let info = p.t_sym_info(info);
		assert_debug_snapshot!(info, @r#"
		(
		    "r",
		    Span {
		        start: 181,
		        end: 194,
		    },
		)
		"#);
	}

	#[test]
	fn doesnt_find_side_effect_import() {
		let alloc = Allocator::new();
		let p = parse!(alloc, "test_data/wp/module.js");
		let info = p.get_imported_var(411104.into());
		assert_eq!(info, None);
	}

	mod module_id {
		use super::*;

		#[test]
		fn parses_module_id() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/module.js");
			let id = p.get_module_id();

			assert_eq!(id, Some(ModuleId(317269)));
		}

		#[test]
		fn fails_to_parse_malformed_module_id() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/bad/badModule1.js");
			let id = p.get_module_id();
			assert_eq!(id, None);
		}

		#[test]
		fn fails_to_parse_missing_module_id() {
			let alloc = Allocator::new();
			let p = parse!(alloc, "test_data/wp/bad/badModule2.js");
			let id = p.get_module_id();
			assert_eq!(id, None);
		}
	}
	mod export_parsing {
		use super::*;
		mod wreq_d {
			use super::*;
			#[test]
			fn simple_modules() {
				let alloc = Allocator::new();
				let p = parse!(alloc, "test_data/wp/module.js");
				let export_map = p.dbg_export_map();
				assert_debug_snapshot!(export_map, @r#"
				{
				    "TB": [
				        "[4:8->4:10)",
				        "[162:13->162:14)",
				    ],
				    "VY": [
				        "[5:8->5:10)",
				        "[183:13->183:14)",
				    ],
				    "ZP": [
				        "[6:8->6:10)",
				        "[87:13->87:14)",
				    ],
				}
				"#);
			}
			#[test]
			fn string_literal_export() {
				let alloc = Allocator::new();
				let p = parse!(alloc, "test_data/wp/wreq.d/simpleString.js");
				let export_map = p.dbg_export_map();
				assert_debug_snapshot!(export_map, @r#"
				{
				    "STRING_EXPORT": "47835198259242069"(
				        [
				            "[5:8->5:21)",
				            "[7:12->7:31)",
				        ],
				    ),
				}
				"#);
			}

			#[test]
			fn object_literal_export() {
				let alloc = Allocator::new();
				let p = parse!(alloc, "test_data/wp/wreq.d/objectExport.js");
				let map = p.dbg_export_map();
				assert_debug_snapshot!(map, @r#"
				{
				    "EO": [
				        "[5:8->5:10)",
				        "[124:13->124:14)",
				    ],
				    "ZP": ExportMap(
				        {
				            "getFormattedName": [
				                "[164:8->164:24)",
				                "[81:13->81:14)",
				            ],
				            "getGlobalName": [
				                "[165:8->165:21)",
				                "[72:13->72:14)",
				            ],
				            "getName": [
				                "[156:8->156:15)",
				                "[53:13->53:14)",
				            ],
				            "getUserTag": [
				                "[159:8->159:18)",
				                "[142:13->142:14)",
				            ],
				            "humanizeStatus": [
				                "[166:8->166:22)",
				                "[90:13->90:14)",
				            ],
				            "isNameConcealed": [
				                "[158:8->158:23)",
				                "[158:25->158:30)",
				            ],
				            "useDirectMessageRecipient": [
				                "[167:8->167:33)",
				                "[147:13->147:14)",
				            ],
				            "useName": [
				                "[157:8->157:15)",
				                "[62:13->62:14)",
				            ],
				            "useUserTag": [
				                "[160:8->160:18)",
				                "[160:20->160:35)",
				            ],
				        },
				    ),
				}
				"#);
			}

			#[test]
			fn object_with_computed_prop() {
				let alloc = Allocator::new();
				let p =
					parse!(alloc, "test_data/wp/wreq.d/computedPropInObj.js");
				let map = p.dbg_export_map();
				assert_debug_snapshot!(map, @r#"
				{
				    "Z": ExportMap(
				        {
				            "n(231338).Et.GET_PLATFORM_BEHAVIORS": ExportMap(
				                {
				                    "handler": [
				                        "[8:12->8:19)",
				                        "[8:21->8:27)",
				                    ],
				                },
				            ),
				        },
				    ),
				}
				"#);
			}

			#[test]
			#[ignore = "todo"]
			fn class_export() {
				let alloc = Allocator::new();
				let p = parse!(alloc, "test_data/wp/wreq.d/classExport.js");
				let map = p.dbg_export_map();
				assert_debug_snapshot!(map, @r#""#);
			}

			#[test]
			fn enum_export() {
				let alloc = Allocator::new();
				let p = parse!(alloc, "test_data/wp/wreq.d/enums.js");
				let map = p.get_export_map();
				// only pick the keys we have tests for in js
				// TODO: Broaden tests in this module
				let mut map2 = map.clone();
				map2.cjs_default = None;
				map2.exports.retain(|k, _| {
					matches!(k.as_str(), "$7" | "$X" | "$n" | "C" | "Cj" | "Si")
				});
				let map2_dumper = ExportMapDumper(&map2, p.source);
				assert_debug_snapshot!(map2_dumper, @r#"
				{
				    "$7": 28(
				        [
				            "[5:8->5:10)",
				            "[385:12->385:14)",
				        ],
				    ),
				    "$X": "1397626558063050855"(
				        [
				            "[7:8->7:10)",
				            "[421:13->421:34)",
				        ],
				    ),
				    "$n": 190(
				        [
				            "[9:8->9:10)",
				            "[739:13->739:16)",
				        ],
				    ),
				    "C": ExportMap(
				        {
				            "PREMIUM_DISCOUNT": 1(
				                [
				                    "[118:12->118:28)",
				                    "[118:31->118:32)",
				                ],
				            ),
				            "PREMIUM_TRIAL": 0(
				                [
				                    "[117:19->117:32)",
				                    "[117:35->117:36)",
				                ],
				            ),
				            "SYM_CJS_DEFAULT": [
				                "[116:8->116:9)",
				            ],
				        },
				    ),
				    "Cj": ExportMap(
				        {
				            "BOX": 2(
				                [
				                    "[701:12->701:15)",
				                    "[701:18->701:19)",
				                ],
				            ),
				            "CAKE": 5(
				                [
				                    "[704:12->704:16)",
				                    "[704:19->704:20)",
				                ],
				            ),
				            "CHEST": 6(
				                [
				                    "[705:12->705:17)",
				                    "[705:20->705:21)",
				                ],
				            ),
				            "COFFEE": 7(
				                [
				                    "[706:12->706:18)",
				                    "[706:21->706:22)",
				                ],
				            ),
				            "CUP": 3(
				                [
				                    "[702:12->702:15)",
				                    "[702:18->702:19)",
				                ],
				            ),
				            "NITROWEEN_STANDARD": 12(
				                [
				                    "[711:12->711:30)",
				                    "[711:33->711:35)",
				                ],
				            ),
				            "SEASONAL_CAKE": 9(
				                [
				                    "[708:12->708:25)",
				                    "[708:28->708:29)",
				                ],
				            ),
				            "SEASONAL_CHEST": 10(
				                [
				                    "[709:12->709:26)",
				                    "[709:29->709:31)",
				                ],
				            ),
				            "SEASONAL_COFFEE": 11(
				                [
				                    "[710:12->710:27)",
				                    "[710:30->710:32)",
				                ],
				            ),
				            "SEASONAL_STANDARD_BOX": 8(
				                [
				                    "[707:12->707:33)",
				                    "[707:36->707:37)",
				                ],
				            ),
				            "SNOWGLOBE": 1(
				                [
				                    "[700:19->700:28)",
				                    "[700:31->700:32)",
				                ],
				            ),
				            "STANDARD_BOX": 4(
				                [
				                    "[703:12->703:24)",
				                    "[703:27->703:28)",
				                ],
				            ),
				            "SYM_CJS_DEFAULT": [
				                "[699:8->699:10)",
				            ],
				        },
				    ),
				    "Si": ExportMap(
				        {
				            "GUILD": "590663762298667008"(
				                [
				                    "[153:10->153:15)",
				                    "[153:18->153:38)",
				                ],
				            ),
				            "LEGACY": "521842865731534868"(
				                [
				                    "[154:10->154:16)",
				                    "[154:19->154:39)",
				                ],
				            ),
				            "NONE": "628379670982688768"(
				                [
				                    "[149:17->149:21)",
				                    "[149:24->149:44)",
				                ],
				            ),
				            "TIER_0": "978380684370378762"(
				                [
				                    "[150:10->150:16)",
				                    "[150:19->150:39)",
				                ],
				            ),
				            "TIER_1": "521846918637420545"(
				                [
				                    "[151:10->151:16)",
				                    "[151:19->151:39)",
				                ],
				            ),
				            "TIER_2": "521847234246082599"(
				                [
				                    "[152:10->152:16)",
				                    "[152:19->152:39)",
				                ],
				            ),
				            "SYM_CJS_DEFAULT": [
				                "[148:8->148:9)",
				            ],
				        },
				    ),
				}
				"#);
			}
		}
		mod e_exports {
			use super::*;
		}
		mod exports {
			use super::*;
		}
		mod stores {
			use super::*;
		}
	}
}
