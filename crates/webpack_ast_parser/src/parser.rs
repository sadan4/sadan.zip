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
			ExportMap,
			ExportMapKey,
			ExportRange,
			ExportValue,
			ExtraData,
			RangeExportMap,
			RangeExportMapValue,
			RangeExportRange,
			RawExportMapValue,
			RawExportRange,
			RawStoreData,
			StoreData,
		},
		types::{WreqD, WreqDExportType},
		util::{
			find_return_identifier,
			flatten_property_access_expression,
			match_export_chain,
		},
	},
	types::ModuleId,
};
use anyhow::Result;
use ast_parser::{
	AstParser,
	ast_kind::IntoAstKind,
	exts::{
		BindingPatternExt,
		ExpressionExt,
		Functionish,
		MemberExpressionExt,
		NumericLiteralExt as _,
		ObjectExpressionExt,
		PropertyKeyExt,
		StatementExt,
	},
	parse,
};
use export_map::RawExportMap;
use oxc::{
	allocator::{Allocator, GetAddress, IntoIn, UnstableAddress},
	ast::{
		AstKind,
		ast::{
			ArrowFunctionExpression,
			CallExpression,
			Class,
			ClassElement,
			Expression,
			IdentifierReference,
			MethodDefinition,
			MethodDefinitionKind,
			NewExpression,
			NumericLiteral,
			ObjectExpression,
			ObjectProperty,
			ObjectPropertyKind,
			Program,
			VariableDeclarator,
		},
	},
	semantic::{Reference, Semantic, SymbolId},
	span::{Atom, GetSpan, SourceType, Span},
};
use smol_str::SmolStr;
use std::{collections::HashMap, iter};
use tracing::{debug, trace, warn};

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
	exports_arg: CacheValue<Option<SymbolId>>,
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
	// FIXME: this is just a direct port from js
	// It should probably be rewritten
	pub fn get_uses_of_import(
		&self,
		m_id: ModuleId,
		export_names: &[ExportMapKey],
	) -> Vec<Span> {
		let Some(wreq) = self.wreq() else {
			return Vec::new();
		};

		let mut uses = Vec::new();

		let iter = self
			.sema
			.symbol_references(wreq)
			.map(Reference::node_id);

		for use_id in iter {
			let Some(p_call) = self.p(use_id).as_call_expression() else {
				continue;
			};
			let p_call_args = &p_call.arguments;
			if p_call_args.len() != 1 {
				continue;
			}
			// continue if the module id does not match
			if p_call_args[0]
				.as_numeric_literal()
				.and_then(NumericLiteral::as_u32)
				.map(ModuleId::from)
				.is_none_or(|id| id != m_id)
			{
				continue;
			}

			match self.p(p_call.node_id()) {
				AstKind::VariableDeclarator(p_call_p_decl) => {
					let Some(name) = p_call_p_decl.id.as_binding_identifier()
					else {
						continue;
					};
					let name_sym_id = name.symbol_id();
					let refs = self
						.sema
						.scoping()
						.get_resolved_reference_ids(name_sym_id);
					// handle things like `var foo = wreq(1), bar = wreq.n(foo);`
					'nmd: {
						if refs.len() == 1 {
							// loc is always a identifier reference because it's a reference to an identifier
							let loc = self
								.n(self
									.sema
									.scoping()
									.get_reference(refs[0])
									.node_id())
								.kind()
								.as_identifier_reference()
								.unwrap();
							let Some(call) = self
								.p(loc.node_id())
								.as_call_expression()
							else {
								break 'nmd;
							};
							if call.arguments.len() != 1 {
								break 'nmd;
							}
							debug_assert!(
								call.arguments[0].address()
									== loc.unstable_address(),
								"how"
							);
							// ensure that the call is `wreq.n(...)`
							let func_expr = &call.callee;
							let Some(func_expr) =
								func_expr.as_static_member_expression()
							else {
								break 'nmd;
							};
							if func_expr.property.name != "n"
								|| func_expr
									.object
									.as_identifier()
									.is_none_or(|wreq_use| {
										!self.cmp_sym(wreq_use, &wreq)
									}) {
								break 'nmd;
							}
							let Some(decl) = self
								.find_parent(
									func_expr.node_id(),
									AstKind::as_variable_declarator,
								)
								.unwrap()
								.id
								.as_binding_identifier()
							else {
								break 'nmd;
							};
							for usage2 in self.refs(decl.symbol_id()) {
								let Some(called_use) =
									self.p(usage2).as_call_expression()
								else {
									continue;
								};
								if export_names.first()
									== Some(&ExportMapKey::Default)
								{
									// a()()
									if self
										.p(called_use.node_id())
										.as_call_expression()
										.is_none()
									{
										continue;
									}
									uses.push(called_use.span());
								} else {
									let Some(called_use_p_access) = self
										.p(called_use.node_id())
										.as_static_member_expression()
									else {
										continue;
									};
									let outer_prop_expr = self.last_parent(
										called_use_p_access.node_id(),
										AstKind::as_static_member_expression,
									).unwrap();
									let full_usage_chain =
										flatten_property_access_expression(
											outer_prop_expr,
										);
									let Some(last_match) = match_export_chain(
										&full_usage_chain,
										export_names,
									) else {
										continue;
									};
									uses.push(last_match.span());
								}
							}
						}
					};
					let refs = refs.iter().copied().map(|ref_id| {
						self.sema
							.scoping()
							.get_reference(ref_id)
							.node_id()
					});
					for loc in refs {
						let Some(loc_p_access) = self
							.p(loc)
							.as_static_member_expression()
						else {
							continue;
						};
						let outer_prop_expr = self
							.last_parent(
								loc_p_access.node_id(),
								AstKind::as_static_member_expression,
							)
							.unwrap();
						let full_usage_chain =
							flatten_property_access_expression(outer_prop_expr);
						let Some(last_match) =
							match_export_chain(&full_usage_chain, export_names)
						else {
							continue;
						};

						uses.push(last_match.span());
					}
				}
				AstKind::StaticMemberExpression(p_call_p_access) => {
					// TODO: fix signature of last_parent so that this .unwrap is not needed
					let outer_prop_expr = self
						.last_parent(
							p_call_p_access.node_id(),
							AstKind::as_static_member_expression,
						)
						.unwrap();
					let full_usage_chain =
						flatten_property_access_expression(outer_prop_expr);
					let Some(last_match) =
						match_export_chain(&full_usage_chain, export_names)
					else {
						continue;
					};
					uses.push(last_match.span());
				}
				_ => {}
			}
		}

		uses
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
	/// TODO: document
	fn exports_arg(&self) -> Option<SymbolId> {
		self.c
			.exports_arg
			.get(|| self.find_webpack_arg(1))
	}
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
	fn get_export_map_raw_wreq_t(&self) -> Option<RawExportMap<'ast>> {
		let mut ret = RawExportMap::default();
		for usage in self.ref_nodes(self.exports_arg()?) {
			let usage = usage.as_identifier_reference().unwrap();
			let Some(export_access) = self
				.p(usage.node_id())
				.as_static_member_expression()
			else {
				continue;
			};
			let Some(export_assignment) = self
				.p(export_access.node_id())
				.as_assignment_expression()
			else {
				continue;
			};
			let key = &export_access.property;
			let key_txt = SmolStr::new(&self.source[key.span()]);
			let export_val = &export_assignment.right;
			let mut val = self.raw_make_export_map_recursive(export_val);
			val.prepend_with(key.into_ast_kind());
			ret.exports.insert(key_txt, val);
		}
		Some(ret)
	}
	fn get_export_map_raw_wreq_d(&self) -> Option<RawExportMap<'ast>> {
		let exports_obj = self.find_wreq_d()?.obj;
		Some(self.raw_make_export_map_object_expression(exports_obj))
	}

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
	fn parse_store_flux_events(
		&self,
		store: &mut RawStoreData<'ast>,
		obj: &'ast ObjectExpression<'ast>,
	) {
		for prop in &obj.properties {
			let Some(prop) = prop.as_property() else {
				warn!(
					"Store flux events has a spread property. This should be handled."
				);
				continue;
			};
			let Some(event_key) = prop.key.as_static_identifier() else {
				warn!(
					"Store flux event key is not a static identifier. This should be handled."
				);
				continue;
			};
			let event_key_txt = SmolStr::new(&self.source[event_key.span()]);
			let event_handler = match &prop.value {
				Expression::Identifier(node) => node.into_ast_kind(),
				Expression::FunctionExpression(func) => {
					func.id.as_ref().map_or_else(
						|| func.into_ast_kind(),
						IntoAstKind::into_ast_kind,
					)
				}
				Expression::ArrowFunctionExpression(func) => {
					func.into_ast_kind()
				}
				_ => {
					warn!(
						"Store flux event handler is not an identifier or functionish. This should be handled."
					);
					continue;
				}
			};
			store
				.flux_events
				.insert(event_key_txt, event_handler);
		}
	}
	/// Try to find the display name of the given store
	/// ## Impl
	/// Display names can be set in three ways
	/// 1. define function
	/// ```js
	/// define(store, "displayName", "MyStore")
	/// ```
	/// 2. Sometimes the bundler will inline the define function
	/// ```js
	/// (i = "displayName")in m ? Object.defineProperty(store, i, {
	///     value: myStoreNameVar,
	///     enumerable: !0,
	///     configurable: !0,
	///     writable: !0
	/// }) : store[i] = myStoreNameVar;
	/// ```
	/// 3. static property
	/// ```js
	/// class minified_store_var extends null /* some store */ {
	///     static displayName = "MyStore";
	/// }
	/// ```
	/// Case 3 is handled in [`Self::raw_make_export_map_store`]
	// TODO: cursed; refactor
	fn try_find_store_name(&self, store_sym_id: SymbolId) -> Option<SmolStr> {
		let iter = self
			.ref_nodes(store_sym_id)
			.filter_map(|node| {
				self.find_parent_limited(
					node.node_id(),
					AstKind::as_call_expression,
					3,
				)
			});
		for usage in iter {
			let args = &usage.arguments;
			if args.len() != 3 {
				continue;
			}
			// TODO: check that args[1] is store_sym_id
			if let Some(define_prop_arg) = args[1].as_string_literal()
				&& define_prop_arg.value == "displayName"
				&& let Some(define_value_arg) = args[2].as_string_literal()
			{
				return Some(SmolStr::new(define_value_arg.value));
			}
			// Object.defineProperty(store)
			// store must be an identifier
			let Some(define_obj_arg) = args[0].as_identifier() else {
				continue;
			};
			// second argument must be an identifier
			let Some(define_prop_arg) = args[1].as_identifier() else {
				continue;
			};
			let Some(define_prop_arg_sym_id) = self.sym_id_of(define_prop_arg)
			else {
				continue;
			};
			// the second arg must be "displayName"
			if !self.is_display_name_prop_key(define_prop_arg_sym_id) {
				continue;
			}
			if !self.cmp_sym(define_obj_arg, &store_sym_id) {
				continue;
			}
			// third arg must be an object literal
			let Some(define_prop_val) = args[2].as_object_expression() else {
				continue;
			};
			let Some(value_prop) = define_prop_val.get_property("value") else {
				continue;
			};
			let Some(value_prop_val) = value_prop
				.value
				.as_identifier()
				.and_then(|ident| self.sym_id_of(ident))
				.and_then(|sym_id| self.is_constant_string(sym_id))
			else {
				continue;
			};
			return Some(SmolStr::new(value_prop_val));
		}
		None
	}
	/// TODO: document
	fn is_constant_string(&self, sym_id: SymbolId) -> Option<Atom<'ast>> {
		let Some(decl) = self
			.sema
			.symbol_declaration(sym_id)
			.kind()
			.as_variable_declarator()
		else {
			return None;
		};
		if let Some(init) = decl.init.as_ref() {
			return init
				.as_string_literal()
				.map(|l| l.value);
		}
		let mut ret = None;
		for reference in self.sema.symbol_references(sym_id) {
			if !reference.is_write() {
				continue;
			}
			// if we're written to more than once, we are not constant
			if ret.is_some() {
				return None;
			}
			ret = self
				.p(reference.node_id())
				.as_assignment_expression()
				.and_then(|assign| assign.right.as_string_literal())
				.map(|s| s.value);
		}
		ret
	}
	fn is_display_name_prop_key(&self, sym_id: SymbolId) -> bool {
		self.is_constant_string(sym_id)
			.is_some_and(|s| s == "displayName")
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
	// TODO: transform extra data?
	fn raw_export_map_to_range_export_map(
		ExportMap {
			exports,
			cjs_default,
			hover,
			extra_data: _,
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
			extra_data: ExtraData::None,
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
		// TODO: we can probably remove this box dyn iter if we use a manual for loop
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

	/// Try to make a raw export map for a discord store
	fn raw_make_export_map_store(
		&self,
		init: &'ast NewExpression<'ast>,
	) -> Option<RawExportMap<'ast>> {
		let init_expr = init.callee.as_identifier()?;
		let store_sym_id = self.sym_id_of(init_expr)?;
		if !matches!(init.arguments.len(), 0 | 2) {
			debug!("Maybe store does not have 0 or 2 ctor args");
			return None;
		}
		let mut ret = RawExportMap {
			extra_data: ExtraData::Store(RawStoreData {
				store: init_expr.into_ast_kind(),
				flux_events: HashMap::new(),
			}),
			..Default::default()
		};
		if init.arguments.len() == 2 {
			// (flux, {/*events obj */}) for Flux Stores
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
		let store_decl = self
			.n(store_decl_id)
			.kind()
			.as_class()?;
		let does_extend = store_decl.super_class.is_some();
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
						"Store has displayName prop but could not resolve display name. This should not happen"
					);
				}
				ret.hover = name.clone();
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
			AstKind::NewExpression(node) => {
				self.raw_make_export_map_new_expression(node)
			}
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

#[cfg(test)]
mod tests;
