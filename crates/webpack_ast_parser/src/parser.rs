mod arg_finder;
mod enum_iife;
mod export_map;
mod types;
mod util;

use crate::{
	bundle::{DefaultModuleCache, IModuleCache},
	cache::{CacheRef, CacheValue},
	parser::{
		enum_iife::EnumIIFEState1_2,
		export_map::{
			ExportMap, ExportRange, ExportValue, RangeExportMap,
			RangeExportMapValue, RangeExportRange, RawExportMapValue,
			RawExportRange,
		},
		types::{WreqD, WreqDExportType},
		util::find_return_identifier,
	},
	types::ModuleId,
};
use anyhow::Result;
use ast_parser::{
	AstParser,
	ast_kind::IntoAstKind,
	exts::{
		BindingPatternExt, ExpressionExt, Functionish, NumericLiteralExt as _,
		StatementExt,
	},
	parse,
};
use export_map::RawExportMap;
use oxc::{
	allocator::Allocator,
	ast::{
		AstKind,
		ast::{
			ArrowFunctionExpression, CallExpression, Class, ClassElement,
			IdentifierReference, MethodDefinition, MethodDefinitionKind,
			NumericLiteral, ObjectExpression, ObjectProperty,
			ObjectPropertyKind, Program, VariableDeclarator,
		},
	},
	semantic::{Semantic, SymbolId},
	span::{GetSpan, SourceType, Span},
};
use smol_str::SmolStr;
use std::iter;
use tracing::debug;

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
	mod_arg: CacheValue<Option<SymbolId>>,
}

impl<'ast> AstParser<'ast> for WebpackAstParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}
}

/// Public API
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
	pub fn get_export_map(&self) -> &RangeExportMap {
		self.c.range_export_map.get(|| {
			let raw = self.get_export_map_raw();
			Self::raw_export_map_to_range_export_map(raw)
		})
	}
}

/// Private API
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
	fn mod_arg(&self) -> Option<SymbolId> {
		self.c
			.mod_arg
			.get(|| self.find_webpack_arg(0))
	}
	// FIXME: implement
	/// TODO: document
	fn get_export_map_raw_module_exports(&self) -> Option<RawExportMap<'ast>> {
		let mod_arg = self.mod_arg()?;
		let mut ret = RawExportMap::default();
		for usage in self.ref_nodes(mod_arg) {
			let usage = usage.as_identifier_reference().unwrap();
			let Some(module_exports_access) = self
				.p(usage.node_id())
				.as_static_member_expression()
			else {
				continue;
			};
			if module_exports_access.property.name != "exports" {
				continue;
			}
			match self.p(module_exports_access.node_id()) {
				AstKind::AssignmentExpression(assign) => {
					let val = &assign.right;
					let new_ret = self.raw_make_export_map_recursive(val);
					match new_ret {
						ExportValue::Map(map) => ret.merge_with(map),
						rng @ ExportValue::Range(_) => {
							assert!(ret.exports.is_empty(), "how???");
							ret.cjs_default = Some(Box::new(rng));
						}
					}
				}
				AstKind::StaticMemberExpression(module_exports_name_access) => {
					let Some(export_assignment) = self
						.p(module_exports_name_access.node_id())
						.as_assignment_expression()
					else {
						continue;
					};
					let export_val = &export_assignment.right;
					let key = &module_exports_name_access.property;
					let key_txt = SmolStr::new(&self.source[key.span()]);
					let val = self.raw_make_export_map_recursive(export_val);
					debug_assert!(
						!ret.exports.contains_key(&key_txt),
						"Duplicate export for key {key_txt}"
					);
					ret.exports.insert(key_txt, val);
				}
				_ => {}
			}
		}
		Some(ret)
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
		merge(self.get_export_map_raw_module_exports());
		ret
	}
	fn get_export_map_raw(&self) -> &RawExportMap<'ast> {
		self.c
			.raw_export_map
			.get_or_default(|| self.impl_get_export_map_raw())
	}
}

/// functions to make the raw export map
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
			self.try_raw_make_export_map_for_enum_iife(node)
		{
			return enum_export.into();
		}
		RawExportRange::from_node(node).into()
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
			.len() == 1
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
			AstKind::Class(node) => self
				.raw_make_export_map_class(node)
				.into(),
			AstKind::MethodDefinition(node) => {
				self.raw_make_export_map_method_definition(node)
			}
			_ => RawExportRange::from(node).into(),
		}
	}
}

#[cfg(test)]
mod tests;
