mod arg_finder;
mod enum_iife;
pub mod export_map;
mod main_func_finder;
mod types;
mod util;

use crate::{
	bundle::{
		self,
		DefaultModuleCache,
		DefaultModuleDepProvider,
		IModuleCache,
		IModuleDepProvider,
	},
	find::{IntlKey, ScoredFindSequence},
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
		},
		types::{
			ReExport,
			ResolvedDefinition,
			SearchElement,
			WreqD,
			WreqDExportType,
		},
		util::{
			filter_export_map,
			find_return_identifier,
			flatten_export_map,
			flatten_property_access_expression,
			get_nested_export_from_map,
			match_export_chain,
			span_to_range,
		},
	},
};
use arrayvec::ArrayString;
use ast_parser::{
	AstParser,
	NodeLocationIndex,
	ast_kind::IntoAstKind,
	cache,
	exts::{
		BindingPatternExt,
		ExpressionExt,
		Functionish,
		MemberExprAccessKind,
		MemberExprRef,
		MemberExpressionExt,
		NumericLiteralExt as _,
		ObjectExpressionExt,
		PropertyKeyExt,
		StatementExt,
	},
	parse_with_tokens,
};
use explorer_types::{
	IncomingModuleDeps,
	ModuleId,
	OutgoingModuleDepsWithLocs,
	SpannedId,
};
use export_map::RawExportMap;
use itertools::Itertools as _;
use miette::{Result, bail};
use miette_ctx::{ErrCtx as _, map_anyhow};
use oxc::{
	allocator::{Allocator, GetAddress, UnstableAddress},
	ast::{
		AstKind,
		ast::{
			Argument,
			ArrowFunctionExpression,
			AssignmentTarget,
			BindingIdentifier,
			CallExpression,
			Class,
			ClassElement,
			Expression,
			ExpressionStatement,
			Function,
			IdentifierReference,
			LogicalOperator,
			MethodDefinition,
			MethodDefinitionKind,
			NewExpression,
			NumericLiteral,
			ObjectExpression,
			ObjectProperty,
			ObjectPropertyKind,
			Program,
			SequenceExpression,
			Statement,
			StaticMemberExpression,
			Str,
			VariableDeclaration,
			VariableDeclarationKind,
			VariableDeclarator,
		},
	},
	parser::{Kind as TK, Token},
	semantic::{NodeId, ReferenceId, Semantic, SymbolId},
	span::{GetSpan, SourceType, Span},
};
use rangemap::RangeSet;
use smol_str::{SmolStr, ToSmolStr as _};
use std::{
	borrow::Cow,
	collections::{HashMap, HashSet},
	fmt::Write,
	iter,
	mem,
	rc::Rc,
};
use tracing::{debug, error, trace, warn};

pub struct WebpackAstParser<'ast> {
	prog: &'ast Program<'ast>,
	sema: Semantic<'ast>,
	source: &'ast str,
	toks: &'ast [Token],
	module_cache: &'ast dyn IModuleCache<'ast>,
	module_dep_provider: &'ast dyn IModuleDepProvider,
	/// Internal cache
	c: Cache<'ast>,
}

#[derive(Default)]
struct Cache<'ast> {
	wreq: cache::Value<Option<SymbolId>>,
	t: cache::Value<Option<SymbolId>>,
	raw_export_map: cache::Ref<RawExportMap<'ast>>,
	range_export_map: cache::Ref<RangeExportMap>,
	wreq_d: cache::Value<Option<WreqD<'ast>>>,
	mod_arg: cache::Value<Option<SymbolId>>,
	exports_arg: cache::Value<Option<SymbolId>>,
	module_id: cache::Value<Option<ModuleId>>,
	does_re_export_whole_module: cache::Value<Option<ModuleId>>,
	modules_that_this_module_requires:
		cache::Ref<Option<OutgoingModuleDepsWithLocs>>,
	num_concatenated_modules: cache::Value<u32>,
	main_func: cache::Value<Option<&'ast Function<'ast>>>,
	node_index: cache::Ref<NodeLocationIndex<'ast>>,
}

impl<'ast> AstParser<'ast> for WebpackAstParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}

	fn node_location_index(&self) -> &cache::Ref<NodeLocationIndex<'ast>> {
		&self.c.node_index
	}
}

/// Public API
impl<'ast> WebpackAstParser<'ast> {
	pub fn try_new(alloc: &'ast Allocator, source: &'ast str) -> Result<Self> {
		let (toks, prog, sema) =
			parse_with_tokens(alloc, source, SourceType::script())?;
		Ok(Self {
			prog,
			sema,
			source,
			toks: toks.into_arena_slice(),
			module_cache: &DefaultModuleCache,
			module_dep_provider: &DefaultModuleDepProvider,
			c: Cache::default(),
		})
	}

	/// takes the module text `src` and returns if the
	/// module text is a webpack module or an extracted find
	pub fn is_webpack_module(src: &str) -> bool {
		src.starts_with("// Webpack Module")
			|| src[0..src.ceil_char_boundary(src.len().min(100))]
				.contains("//OPEN FULL MODULE:")
	}

	/// Returns the number of bytes inserted at the start of `src`, or `0` if
	/// `src` was already a webpack module and nothing was inserted.
	pub fn format_module_header(
		src: &mut String,
		m_id: ModuleId,
		is_find: bool,
	) -> usize {
		const BUF_LEN: usize = 128;
		if Self::is_webpack_module(src) {
			return 0;
		}
		let mut buf = ArrayString::<BUF_LEN>::new_const();
		writeln!(buf, "// Webpack Module {m_id}").unwrap();
		if is_find {
			writeln!(buf, "//OPEN FULL MODULE: {m_id}").unwrap();
		}
		writeln!(buf, "//EXTRACTED WEBPACK MODULE {m_id}").unwrap();
		writeln!(buf, "0,").unwrap();
		src.insert_str(0, &buf);
		buf.len()
	}

	pub const fn get_source(&self) -> &'ast str {
		self.source
	}

	pub fn set_module_cache(
		&mut self,
		module_cache: &'ast dyn IModuleCache<'ast>,
	) {
		self.module_cache = module_cache;
	}

	pub fn set_module_dep_provider(
		&mut self,
		module_dep_provider: &'ast dyn IModuleDepProvider,
	) {
		self.module_dep_provider = module_dep_provider;
	}

	pub fn get_module_id(&self) -> Option<ModuleId> {
		self.c
			.module_id
			.get(|| self.get_module_id_impl())
	}
	pub fn get_export_map(&self) -> &RangeExportMap {
		self.c.range_export_map.get(|| {
			let raw = self.get_export_map_raw();
			Self::raw_export_map_to_range_export_map(raw)
		})
	}
	pub fn get_uses_of_import(
		&self,
		m_id: ModuleId,
		export_names: &[ExportMapKey],
	) -> Vec<Span> {
		let Some(wreq) = self.wreq() else {
			return Vec::new();
		};

		let mut uses = Vec::new();

		for wreq_ref in self.refs(wreq) {
			let Some(require_call) =
				self.match_wreq_require_call(wreq_ref, m_id)
			else {
				continue;
			};

			match self.p(require_call.node_id()) {
				// `var foo = wreq(m_id);` - chase uses of `foo`
				AstKind::VariableDeclarator(decl) => {
					let Some(name) = decl.id.as_binding_identifier() else {
						continue;
					};
					let binding_refs = self
						.sema
						.scoping()
						.get_resolved_reference_ids(name.symbol_id());

					// `var foo = wreq(m), bar = wreq.n(foo);`
					// also chase uses through `bar`
					if let Some(n_alias) =
						self.try_resolve_wreq_n_alias(binding_refs, wreq)
					{
						self.collect_uses_via_wreq_n_alias(
							n_alias,
							export_names,
							&mut uses,
						);
					}

					for ref_id in binding_refs {
						let ref_node = self
							.sema
							.scoping()
							.get_reference(*ref_id)
							.node_id();
						let Some(access) = self
							.p(ref_node)
							.as_static_member_expression()
						else {
							continue;
						};
						if let Some(span) =
							self.match_outer_access_chain(access, export_names)
						{
							uses.push(span);
						}
					}
				}
				// `wreq(m_id).foo.bar` - used inline
				AstKind::StaticMemberExpression(access) => {
					if let Some(span) =
						self.match_outer_access_chain(access, export_names)
					{
						uses.push(span);
					}
				}
				_ => {}
			}
		}

		uses
	}
	// TODO: use custom error codes with thiserror
	pub fn generate_references(
		&self,
		pos: u32,
	) -> Result<Vec<bundle::Reference<'ast>>> {
		let Some(self_module_id) = self.get_module_id() else {
			bail!(
				"Could not find module id of module to search for references of."
			);
		};
		let module_exports = self.get_export_map();
		let where_ = self.get_modules_that_require_this_module()?;
		let mut locs = Vec::new();
		// TODO: construct a new map from a ref instead of cloning
		let filtered_export_map =
			filter_export_map(module_exports.clone(), pos);
		let exported_names = flatten_export_map(filtered_export_map, None);

		for mut export_name in exported_names {
			let mut seen: HashMap<ModuleId, HashSet<ModuleId>> = HashMap::new();
			// below fixme is copied verbatim from js. it might not be valid
			// FIXME: this is a workaround for a bug in getUsesOfImport where it doesn't properly hand SYM_CJS_DEFAULT
			if export_name.len() > 1 && export_name.last().unwrap().is_default()
			{
				export_name.pop();
			}

			let mut left = where_
				.sync
				.iter()
				.map(|x| {
					SearchElement {
						module_id: *x,
						imported_id: self_module_id,
						// TODO: make this cow?
						export_name: export_name.clone(),
					}
				})
				.collect_vec();
			while let Some(cur) = left.pop() {
				let SearchElement {
					module_id,
					imported_id,
					export_name,
				} = cur;
				if seen
					.get(&imported_id)
					.is_some_and(|s| s.contains(&module_id))
				{
					continue;
				}
				seen.entry(imported_id)
					.or_default()
					.insert(module_id);
				let parser = match self
					.module_cache
					.get_module_parser(self, module_id, None)
				{
					Ok(parser) => parser,
					Err(e) => {
						warn!(
							"Failed to get parser for module id {module_id}. Cause: {e:?}"
						);
						continue;
					}
				};
				let uses = parser.get_uses_of_import(imported_id, &export_name);
				// FIXME: support nested re-exports
				let exported_as = parser.does_re_export_from_import(
					imported_id,
					export_name[0].clone(),
				);

				if let Some(exported_as) = exported_as
					&& let Ok(where_) =
						parser.get_modules_that_require_this_module()
				{
					left.extend(
						where_
							.sync
							.iter()
							.map(|x| SearchElement {
								module_id: *x,
								imported_id: parser.get_module_id().unwrap(),
								export_name: vec![exported_as.clone()],
							}),
					);
				}
				let maybe_file_path = self
					.module_cache
					.get_module_filepath(module_id);
				locs.extend(uses.iter().map(|&range| {
					maybe_file_path.clone().map_or(
						bundle::Reference {
							range,
							module_id,
							location: bundle::Location::Inline(self.source),
						},
						|file_path| bundle::Reference {
							location: bundle::Location::Path(file_path),
							module_id,
							range,
						},
					)
				}));
			}
		}

		Ok(locs)
	}
	pub fn get_modules_that_this_module_requires(
		&self,
	) -> Option<&OutgoingModuleDepsWithLocs> {
		self.c
			.modules_that_this_module_requires
			.get(|| self.get_modules_that_this_module_requires_impl())
			.as_ref()
	}
	fn get_modules_that_this_module_requires_impl(
		&self,
	) -> Option<OutgoingModuleDepsWithLocs> {
		let wreq = self.wreq()?;
		// TODO: merge these loops to avoid two iterations
		let sync = self
			.refs(wreq)
			.filter_map(|usage| {
				let p_call =
					self.find_parent(usage, AstKind::as_call_expression)?;
				let args = &p_call.arguments;
				if args.len() != 1 {
					return None;
				}
				let id =
					ModuleId::from(args[0].as_numeric_literal()?.as_u32()?);
				let span = args[0].span();

				Some(SpannedId { id, span })
			})
			.collect();
		// TODO: implement lazy require parsing
		let lazy = Vec::new();
		Some(OutgoingModuleDepsWithLocs { sync, lazy })
	}
	pub fn get_modules_that_require_this_module(
		&self,
	) -> Result<Rc<IncomingModuleDeps>> {
		let module_id = self
			.get_module_id()
			.context("Module ID not found")?;
		self.module_dep_provider
			.get_module_deps(module_id)
			.map_err(map_anyhow)
	}
	/// Figure out if this module re-exports another given the module id of the other and
	/// the name of the export from the other module.
	///
	/// `module_id` the module id that `export_name` is from
	/// `export_name` the name of the re-exported export
	pub fn does_re_export_from_import(
		&self,
		module_id: ModuleId,
		export_name: ExportMapKey,
	) -> Option<ExportMapKey> {
		if self
			.does_re_export_whole_module()
			.is_some()
		{
			// how???
			debug_assert_eq!(
				self.does_re_export_whole_module(),
				Some(module_id)
			);
			return Some(export_name);
		}
		let decl = self.get_imported_var(module_id)?;
		let mut maybe_re_exports = self
			.get_export_map_raw()
			.exports
			.iter()
			.filter(|(_, v)| {
				let Ok(v) = v.try_unwrap_range_ref() else {
					return false;
				};
				let Some(v) = v.first() else {
					return false;
				};
				// TODO: why are we taking the first one
				match v {
					AstKind::IdentifierReference(node) => {
						self.cmp_sym(*node, &decl)
					}
					AstKind::StaticMemberExpression(_) => todo!(),
					v => {
						warn!("Unhandled type for reExport: {v:?}");
						false
					}
				}
			})
			.map(|(k, _)| k)
			.collect_vec();
		if maybe_re_exports.len() != 1 {
			if maybe_re_exports.len() > 1 {
				error!(
					"Found more than one reExport for wreq({module_id}).{export_name:?}"
				);
			}
			return None;
		}
		Some(
			maybe_re_exports
				.swap_remove(0)
				.to_smolstr()
				.into(),
		)
	}
	pub fn generate_definitions(
		&self,
		pos: u32,
	) -> Result<Vec<bundle::Definition<'ast>>> {
		let selected_node = self.get_node_at(pos);
		if let Some(num_lit) = selected_node.as_numeric_literal() {
			return self.generate_direct_module_definition(num_lit);
		}
		let ResolvedDefinition {
			parser,
			export_names,
			raw_export_names: _,
		} = self
			.resolve_definition(selected_node)
			.with_context(|| {
				format!(
					"Failed to resolve definition of selected node at {:?} {}",
					selected_node.span(),
					if cfg!(debug_assertions) {
						selected_node.debug_name()
					} else {
						Cow::Borrowed("")
					}
				)
			})?;
		let range = if export_names.is_empty() {
			Span::default()
		} else {
			parser.find_export_location(&export_names)
		};
		let module_id = parser
			.get_module_id()
			.context("Failed to get module id from parser of export")?;
		Ok(vec![bundle::Definition {
			range,
			location: bundle::Location::Inline(parser.source),
			module_id,
		}])
	}
	pub fn get_hover_text(&self, keys: &[ExportMapKey]) -> Option<SmolStr> {
		let mut cur = self.get_export_map();
		let mut last = None;
		for key in keys {
			let Some(val) = cur.get(key) else {
				break;
			};
			match val {
				ExportValue::Range(ExportRange(_, hover)) => {
					last = last.or_else(|| hover.clone());
					break;
				}
				ExportValue::Map(map) => {
					if let Some(hover) = &map.hover {
						last = Some(hover.clone());
					} else if let Some(hover) = map
						.cjs_default
						.as_deref()
						.and_then(ExportValue::get_hover)
					{
						debug_assert!(
							false,
							"cjs default hover should be on the parent export map"
						);
						last = Some(hover.clone());
					} else {
						last = None;
					}
					cur = map;
				}
			}
		}
		last
	}
	/// FIXME: extract (Span, `SmolStr`) into a separate struct;
	pub fn generate_hover(&self, pos: u32) -> Result<Option<(Span, SmolStr)>> {
		let selected_node = self.get_node_at(pos);
		let ResolvedDefinition {
			parser,
			export_names,
			raw_export_names,
		} = match self.resolve_definition(selected_node) {
			Ok(it) => it,
			Err(err) => {
				trace!("Failed to resolve definition for hover: {err}");
				return Ok(None);
			}
		};
		if export_names.is_empty() {
			return Ok(None);
		}
		let Some(hover) = parser.get_hover_text(&export_names) else {
			return Ok(None);
		};
		let range = raw_export_names.last().unwrap().span();
		Ok(Some((range, hover)))
	}

	pub fn get_i18n_key_at(&self, pos: u32) -> Option<(Span, SmolStr)> {
		let node = self.get_node_at(pos);
		let key = match node {
			AstKind::IdentifierReference(id) => id.name.to_smolstr(),
			AstKind::IdentifierName(id) => id.name.to_smolstr(),
			AstKind::StringLiteral(s) => s.value.to_smolstr(),
			_ => return None,
		};

		if key.len() != 6 {
			return None;
		}

		let parent = self.p(node.node_id());
		match parent {
			AstKind::StaticMemberExpression(_)
			| AstKind::ComputedMemberExpression(_) => Some((node.span(), key)),
			_ => None,
		}
	}

	/// Includes the "base" module
	pub fn num_concatenated_modules(&self) -> u32 {
		self.c.num_concatenated_modules.get(|| {
			self.count_num_concatentated_modules()
				.unwrap_or(0)
		})
	}
	/// Attempt to generate a unique string for this module
	///
	/// nothing here is guaranteed to be unique, these are just the best candidates
	///
	/// they need to be filtered for uniqueness by the caller
	pub fn generate_finds(&self) -> Vec<ScoredFindSequence> {
		self.impl_generate_finds()
	}

	/// Collect every i18n/intl key referenced anywhere in the module, together
	/// with the [`Span`] at which each is used.
	///
	/// Keys are returned in source order and may contain duplicates when the
	/// same key is used in multiple places. Each key's original (unhashed)
	/// message name is resolved when present in the embedded key mapping.
	///
	/// Unlike [`Self::get_i18n_key_at`] (which is loose enough for a
	/// cursor-on-key hover), this only matches keys that are passed as an
	/// argument to a Discord intl formatting call — `intl.string(<key>)` /
	/// `intl.format(<key>)` (possibly nested inside a ternary etc). This keeps
	/// real keys regardless of the messages accessor used (`x.t.KEY`,
	/// `x.default.KEY`, ...) while excluding unrelated 6-char member accesses
	/// (`Object.keys`, `arguments.length`, `.concat`, `.string`, ...).
	pub fn get_intl_keys(&self) -> Vec<(Span, IntlKey)> {
		self.toks
			.iter()
			.filter_map(|t| {
				let pos = t.span().start;
				if !self.is_intl_format_arg(pos) {
					return None;
				}
				let (span, hashed) = self.get_i18n_key_at(pos)?;
				let unhashed = crate::intl::resolve_unhashed_key(&hashed);
				Some((span, IntlKey { hashed, unhashed }))
			})
			.collect()
	}

	/// Attempt to determine if the current module is an intl module
	pub fn is_intl_module(&self) -> bool {
		let Some(mf) = self.get_main_func() else {
			return false;
		};
		// function () { ... }
		let [es] = mf
			.body
			.as_ref()
			.unwrap()
			.statements
			.as_slice()
		else {
			return false;
		};
		// expr;
		let Statement::ExpressionStatement(es) = es else {
			return false;
		};
		// ... = ...;
		let Expression::AssignmentExpression(assign) = &es.expression else {
			return false;
		};
		// foo.bar = ...;
		let AssignmentTarget::StaticMemberExpression(module_exports_use) =
			&assign.left
		else {
			return false;
		};
		let Expression::Identifier(module_use) = &module_exports_use.object
		else {
			return false;
		};
		let Expression::CallExpression(json_parse_intl) = &assign.right else {
			return false;
		};
		let Expression::StaticMemberExpression(json_parse) =
			&json_parse_intl.callee
		else {
			return false;
		};
		let Expression::Identifier(json_ref) = &json_parse.object else {
			return false;
		};
		if !self
			.sema
			.is_reference_to_global_variable(json_ref)
			|| json_ref.name != "JSON"
			|| json_parse.property.name != "parse"
		{
			return false;
		}
		let [Argument::StringLiteral(intl)] =
			json_parse_intl.arguments.as_slice()
		else {
			return false;
		};
		let Some(module) = self.mod_arg() else {
			return false;
		};
		if !self.cmp_sym(module_use.as_ref(), &module)
			|| module_exports_use.property.name != "exports"
		{
			return false;
		}
		let intl = intl.value.as_str();
		Self::is_valid_intl_json(intl)
	}
}

/// Private API
#[expect(clippy::multiple_inherent_impl)]
impl<'ast> WebpackAstParser<'ast> {
	/// Whether the key node at `pos` is used as an argument to a Discord intl
	/// formatting call, i.e. the nearest enclosing [`CallExpression`] has a
	/// callee of the form `<x>.string` / `<x>.format*` and the key lies within
	/// the call's arguments (not the callee itself).
	fn is_intl_format_arg(&self, pos: u32) -> bool {
		let node = self.get_node_at(pos);
		let Some(call) =
			self.find_parent(node.node_id(), AstKind::as_call_expression)
		else {
			return false;
		};
		// the key must be inside the arguments, not part of the callee
		// (e.g. the `string` in `intl.string(...)` is itself a 6-char member)
		let callee_span = call.callee.span();
		let key_span = node.span();
		if key_span.start >= callee_span.start
			&& key_span.end <= callee_span.end
		{
			return false;
		}
		let Expression::StaticMemberExpression(m) = &call.callee else {
			return false;
		};
		let method = m.property.name.as_str();
		method == "string" || method.starts_with("format")
	}

	fn is_valid_intl_json(json: &str) -> bool {
		use serde_json::{Value, from_str};
		let Ok(Value::Object(obj)) = from_str(json) else {
			return false;
		};
		// naive impl.
		// TODO: check https://github.com/discord/discord-intl/tree/main and look into the format exported by the tool
		for (_, v) in obj {
			if !v.is_array() {
				return false;
			}
		}
		true
	}
	fn get_main_func(&self) -> Option<&'ast Function<'ast>> {
		self.c
			.main_func
			.get(|| main_func_finder::find(self))
	}
	/// checks if the expression is `wreq` or `wreq.n`
	fn is_import_callee(
		&self,
		wreq: SymbolId,
		expr: &'ast Expression<'ast>,
	) -> bool {
		match expr {
			Expression::Identifier(ident) => {
				self.cmp_sym(ident.as_ref(), &wreq)
			}
			Expression::StaticMemberExpression(access) => {
				access
					.object
					.as_identifier()
					.is_some_and(|id| self.cmp_sym(id, &wreq))
					&& access.property.name == "n"
			}
			_ => false,
		}
	}
	/// checks if `node` is a decl of webpack imports
	fn is_import_decl(&self, node: &'ast VariableDeclaration<'ast>) -> bool {
		if node.kind != VariableDeclarationKind::Var {
			// webpack import blocks use var, not let
			return false;
		}
		let Some(wreq) = self.wreq() else {
			// we cant import anything if we dont have wreq
			return false;
		};
		let mut last_decl_id = SymbolId::MAX_INDEX;
		let mut iter = node
			.declarations
			.iter()
			// webpack will declare extra variables that will be used as empty first
			// eg:
			// var a, b, c, d = wreq(0);
			.skip_while(|d| d.init.is_none())
			.peekable();
		if iter.peek().is_none() {
			return false;
		}
		for decl in iter {
			// _ = n(000000)
			// OR
			// b = n.n(a)
			// where a was the previous declaration
			let Some(init) = decl.init.as_ref() else {
				return false;
			};
			// only plain idents are bound to imports
			let Some(decl_ident) = decl.id.as_binding_identifier() else {
				return false;
			};
			let Expression::CallExpression(call) = init else {
				return false;
			};
			if !self.is_import_callee(wreq, &call.callee) {
				return false;
			}
			// is the argument valid
			match call.arguments.as_slice() {
				[Argument::NumericLiteral(_)] => {
					last_decl_id = decl_ident.symbol_id().into();
				}
				[Argument::Identifier(ident)] => {
					if last_decl_id != SymbolId::MAX_INDEX
						// SAFETY: it's not MAX_INDEX
						// and we only ever set it to MAX_INDEX as a sentinel value 
						// or an already valid symbol id
						&& !self.cmp_sym(ident.as_ref(), &unsafe {
							SymbolId::from_usize_unchecked(last_decl_id)
						}) {
						return false;
					}
				}
				_ => {
					return false;
				}
			}
		}
		true
	}

	/// Webpack will insert side effect imports as seen below
	///
	/// returns true if `stmt` is a side effect import statement
	///
	/// ```ts
	/// import foo from "foo";
	/// import "bar";
	/// import baz from "baz";
	/// // Turns into
	/// var foo = wreq(0); // importing foo
	/// wreq(1); // side effect import bar
	/// var baz = wreq(2); // importing baz
	/// ```
	fn is_side_effect_import_stmt(
		&self,
		stmt: &'ast ExpressionStatement<'ast>,
	) -> bool {
		// wreq(...); must be a call expr
		let Expression::CallExpression(call) = &stmt.expression else {
			return false;
		};
		let Some(wreq) = self.wreq() else {
			return false;
		};
		// it must be a call on a plain identifier
		// webpack will do an indirect call on imports to change the `this` value
		// eg: `(0, foo.default)(...)`, which is not a side effect import
		let Expression::Identifier(wreq_ref) = &call.callee else {
			return false;
		};
		let [Argument::NumericLiteral(_)] = call.arguments.as_slice() else {
			return false;
		};
		self.cmp_sym(wreq_ref.as_ref(), &wreq)
	}

	fn count_num_concatentated_modules(&self) -> Option<u32> {
		let main_func = self.get_main_func()?;
		let mut count = 0;
		let mut last_was_import_block = false;
		for stmt in &main_func
			.body
			.as_ref()
			.unwrap()
			.statements
		{
			if let Statement::VariableDeclaration(decl) = stmt
				&& self.is_import_decl(decl)
			{
				if !last_was_import_block {
					count += 1;
				}
				last_was_import_block = true;
			} else if last_was_import_block
				&& let Statement::ExpressionStatement(stmt) = stmt
				&& self.is_side_effect_import_stmt(stmt)
			{
				// do nothing if we find a side effect import statement in the middle of an import block
			} else {
				last_was_import_block = false;
			}
		}
		Some(count)
	}
	/// Given a reference to `wreq`, returns the `wreq(m_id)` call expression the
	/// reference is the callee of — or `None` if it isn't being used to require `m_id`.
	fn match_wreq_require_call(
		&self,
		wreq_ref: NodeId,
		m_id: ModuleId,
	) -> Option<&'ast CallExpression<'ast>> {
		let call = self.p(wreq_ref).as_call_expression()?;
		if call.arguments.len() != 1 {
			return None;
		}
		let arg_id = call.arguments[0]
			.as_numeric_literal()
			.and_then(NumericLiteral::as_u32)
			.map(ModuleId::from)?;
		(arg_id == m_id).then_some(call)
	}

	/// Detect the `var foo = wreq(m), bar = wreq.n(foo);` pattern.
	///
	/// Given the references to `foo`, if its only reference is the argument to a
	/// `wreq.n(...)` call whose result is bound to a variable, return the symbol of
	/// that variable (`bar`).
	fn try_resolve_wreq_n_alias(
		&self,
		binding_refs: &[ReferenceId],
		wreq: SymbolId,
	) -> Option<SymbolId> {
		let [ref_id] = binding_refs else {
			return None;
		};
		// always an identifier reference because it's a reference to an identifier
		let loc = self
			.n(self
				.sema
				.scoping()
				.get_reference(*ref_id)
				.node_id())
			.kind()
			.as_identifier_reference()
			.unwrap();
		let call = self
			.p(loc.node_id())
			.as_call_expression()?;
		if call.arguments.len() != 1 {
			return None;
		}
		debug_assert!(
			call.arguments[0].address() == loc.unstable_address(),
			"how"
		);
		// ensure that the call is `wreq.n(...)`
		let callee = call
			.callee
			.as_static_member_expression()?;
		if callee.property.name != "n" {
			return None;
		}
		if !self.cmp_sym(callee.object.as_identifier()?, &wreq) {
			return None;
		}
		self.find_parent(callee.node_id(), AstKind::as_variable_declarator)
			.unwrap()
			.id
			.as_binding_identifier()
			.map(BindingIdentifier::symbol_id)
	}

	/// Collect uses through the `bar` variable in `var bar = wreq.n(foo);`.
	///
	/// `wreq.n` returns a getter, so:
	/// - For the default export, usages look like `bar()()`.
	/// - Otherwise, treat `bar.x.y...` as a normal member-access chain.
	fn collect_uses_via_wreq_n_alias(
		&self,
		alias: SymbolId,
		export_names: &[ExportMapKey],
		uses: &mut Vec<Span>,
	) {
		let want_default = export_names.first() == Some(&ExportMapKey::Default);
		for usage in self.refs(alias) {
			let Some(call) = self.p(usage).as_call_expression() else {
				continue;
			};
			if want_default {
				// `bar()()`
				if self
					.p(call.node_id())
					.as_call_expression()
					.is_some()
				{
					uses.push(call.span());
				}
			} else if let Some(access) = self
				.p(call.node_id())
				.as_static_member_expression()
				&& let Some(span) =
					self.match_outer_access_chain(access, export_names)
			{
				uses.push(span);
			}
		}
	}

	/// Walks outward through any chained static member accesses starting at
	/// `inner_access`, then matches the resulting chain against `export_names`,
	/// returning the span of the final matching segment.
	fn match_outer_access_chain(
		&self,
		inner_access: &'ast StaticMemberExpression<'ast>,
		export_names: &[ExportMapKey],
	) -> Option<Span> {
		// inner_access itself satisfies the predicate, so last_parent never returns None
		let outer = self
			.last_parent(
				inner_access.node_id(),
				AstKind::as_static_member_expression,
			)
			.unwrap();
		let chain = flatten_property_access_expression(outer);
		match_export_chain(&chain, export_names).map(GetSpan::span)
	}

	fn does_re_export_from_export(
		&self,
		export_name: &[ExportMapKey],
	) -> Option<ReExport<'ast>> {
		let map = self.get_export_map_raw();
		let exp = get_nested_export_from_map(export_name, map)?;
		let last = exp.last()?;
		let last = last.as_static_member_expression()?;
		let (imported, chain) = flatten_property_access_expression(last);
		let imported = imported.as_identifier()?;
		let imported_sym_id = self.sym_id_of(imported)?;
		if chain.is_empty() {
			debug_assert!(false, "how??");
			return None;
		}
		let imported_id = self.get_module_id_for_import(imported_sym_id)?;
		let ret = ReExport {
			import_source_id: imported_id,
			export_names: chain,
		};
		Some(ret)
	}
	fn resolve_definition(
		&self,
		selected_node: AstKind<'ast>,
	) -> Result<ResolvedDefinition<'ast>> {
		let access_chain = self
			.find_parent(selected_node.node_id(), MemberExprRef::from_node)
			.context("Could not find access chain")?;
		let (required_module, names) =
			flatten_property_access_expression(access_chain);
		// TODO: should this check if requiredModule.expression is wreq
		// i think probably not, no real need
		let module_id = if let Some(call) = required_module.as_call_expression()
			&& call.arguments.len() == 1
		{
			call.arguments[0]
				.as_numeric_literal()
				.and_then(NumericLiteral::as_u32)
				.map(ModuleId::from)
		} else if let Some(ident) = required_module.as_identifier() {
			self.sym_id_of(ident)
				.and_then(|sym_id| self.get_module_id_for_import(sym_id))
		} else {
			None
		};
		let module_id =
			module_id.context("Failed to get module id from access chain")?;

		let mut cur = self.try_get_module_parser(module_id)?;
		if cfg!(debug_assertions) && cur.get_module_id() != Some(module_id) {
			warn!(ast=?module_id, parser=?cur.get_module_id(),"Parser did not return the same module id as the AST.");
		}
		debug_assert!(!names.is_empty(), "document how");
		if names.is_empty() {
			return Ok(ResolvedDefinition {
				parser: cur,
				export_names: vec![],
				raw_export_names: vec![],
			});
		}
		let mut raw_names: Vec<MemberExprAccessKind<'ast>> = names;
		let mut mapped_names = raw_names
			.iter()
			.map_while(|a| a.try_unwrap_static().ok())
			.map(|ident| &ident.name)
			.map(ExportMapKey::from_str)
			.collect_vec();
		raw_names.truncate(mapped_names.len());
		// TODO: extract updating mapped_names into a helper lambda
		loop {
			// check for an explicit re-export before falling back to checking for a whole module re-export
			let ret = cur.does_re_export_from_export(&mapped_names);
			let Some(ReExport {
				import_source_id,
				export_names,
			}) = ret
			else {
				let whole_module_export_id = cur.does_re_export_whole_module();
				if let Some(whole_module_export_id) = whole_module_export_id {
					let maybe_module =
						self.try_get_module_parser(whole_module_export_id);
					match maybe_module {
						Ok(module) => {
							cur = module;
							continue;
						}
						Err(e) => {
							warn!("BUG: {e:?}");
						}
					}
				}
				break;
			};
			raw_names = export_names;
			mapped_names = raw_names
				.iter()
				.map_while(|a| a.try_unwrap_static().ok())
				.map(|ident| &ident.name)
				.map(ExportMapKey::from_str)
				.collect_vec();
			raw_names.truncate(mapped_names.len());
			cur = self
				.try_get_module_parser(import_source_id)
				.context("Failed to get module parser")?;
		}
		Ok(ResolvedDefinition {
			export_names: mapped_names,
			raw_export_names: raw_names,
			parser: cur,
		})
	}
	fn find_export_location(&self, export_names: &[ExportMapKey]) -> Span {
		let mut map = self.get_export_map();
		let mut range = Span::default();
		for key in export_names {
			let Some(val) = map.get(key) else {
				break;
			};
			match val {
				ExportValue::Range(rng) => {
					if let Some(r) = rng.last() {
						range = *r;
					} else {
						error!("Empty export range");
					}
					break;
				}
				ExportValue::Map(new_map) => {
					if let Some(rng) = new_map
						.cjs_default
						.as_deref()
						.and_then(|a| a.try_unwrap_range_ref().ok())
					{
						if let Some(r) = rng.last() {
							range = *r;
						} else {
							error!("Empty export range");
						}
					}
					map = new_map;
				}
			}
		}
		range
	}
	fn try_get_module_parser(&self, module_id: ModuleId) -> Result<Rc<Self>> {
		self.module_cache
			.get_latest_module_parser(self, module_id)
			.map_err(map_anyhow)
	}
	/// Gets the [`ModuleId`] from a require by the returned symbol id
	/// ```js
	/// var mod = wreq(123);
	/// ```
	/// given the symbol id of `mod`, this function would return `Some(ModuleId(123))`
	fn get_module_id_for_import(&self, sym_id: SymbolId) -> Option<ModuleId> {
		let decl = self
			.sema
			.symbol_declaration(sym_id)
			.kind()
			.as_variable_declarator()?;
		let init = decl
			.init
			.as_ref()?
			.as_call_expression()?;
		// make sure init is a call to wreq
		if !self.cmp_sym(init.callee.as_identifier()?, &self.wreq()?) {
			return None;
		}
		let args = &init.arguments;
		if args.len() != 1 {
			return None;
		}
		args[0]
			.as_numeric_literal()
			.and_then(NumericLiteral::as_u32)
			.map(ModuleId::from)
	}
	fn generate_direct_module_definition(
		&self,
		node: &'ast NumericLiteral<'ast>,
	) -> Result<Vec<bundle::Definition<'ast>>> {
		let call = self
			.p(node.node_id())
			.as_call_expression()
			.context("number parent is not a call")?;
		if call.arguments.len() != 1 {
			bail!("expected module it to be the only argument");
		}
		let func = call
			.callee
			.as_identifier()
			.context("expected callee to be an ident")?;
		if !self.cmp_sym(
			func,
			&self
				.wreq()
				.context("couldnt find wreq")?,
		) {
			bail!("expected callee to be wreq");
		}
		let module_id = node
			.as_u32()
			.context("number is not a valid module it")?
			.into();
		let file_path = self
			.module_cache
			.get_module_filepath(module_id)
			.context("Could not get module filepath")?;
		let ret = vec![bundle::Definition {
			range: Span::default(),
			module_id,
			location: bundle::Location::Path(file_path),
		}];
		Ok(ret)
	}
	fn get_module_id_impl(&self) -> Option<ModuleId> {
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
	/// Returns the symbol id for the `module` argument
	///
	/// This is oftentimes the ident `e`
	///
	/// ```js
	/// 0,
	/// function(module, exports, wreq) {
	/// // ...
	/// }
	/// ```
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
					// bail out if we are on the rhs
					// eg: `e.exports.default = e.exports`
					if assign.left.address()
						!= module_exports_access.unstable_address()
					{
						continue;
					}
					let val = &assign.right;
					let new_ret = self.raw_make_export_map_recursive(val);
					match new_ret {
						ExportValue::Map(map) => ret.merge_with(map),
						rng @ ExportValue::Range(_) => {
							if !ret.exports.is_empty() {
								debug!(
									"module.exports in module id {:?} is assigned to more than once",
									self.get_module_id()
								);
								continue;
							}
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
					// bail out of we are on the rhs
					// eg `e.exports.bar = e.exports.foo`
					if module_exports_name_access.unstable_address()
						!= export_assignment.left.address()
					{
						continue;
					}
					let export_val = &export_assignment.right;
					let key = &module_exports_name_access.property;
					let key_txt = SmolStr::new(&self.source[key.span()]);
					let val = self.raw_make_export_map_recursive(export_val);
					if ret.exports.contains_key(&key_txt) {
						debug!(
							"module.exports.{key_txt} is assigned to more than once in module id {:?}",
							self.get_module_id()
						);
						continue;
					}
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
			return Some(WreqD {
				_call: call,
				_exports: exports,
				obj,
			});
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
	fn is_constant_string(&self, sym_id: SymbolId) -> Option<Str<'ast>> {
		let decl = self
			.sema
			.symbol_declaration(sym_id)
			.kind()
			.as_variable_declarator()?;
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
	fn does_re_export_whole_module_impl(&self) -> Option<ModuleId> {
		let mod_arg = self.mod_arg()?;
		for use_ in self.wreq_uses()? {
			let Some(assignment) = self
				.find_parent(use_.node_id(), AstKind::as_assignment_expression)
			else {
				continue;
			};

			let Some(lhs) = assignment
				.left
				.as_static_member_expression()
			else {
				continue;
			};
			let (module, exports_arr) = flatten_property_access_expression(lhs);
			let Some(module) = module.as_identifier() else {
				continue;
			};
			if !self.cmp_sym(module, &mod_arg) {
				continue;
			}
			debug_assert!(
				exports_arr.len() == 1,
				"chain should always have len 1"
			);
			if exports_arr
				.last()
				.and_then(|e| e.try_unwrap_static().ok())
				.is_none_or(|e| e.name != "exports")
			{
				continue;
			}
			let rhs = assignment.right.as_call_expression()?;
			if rhs.callee.address() != use_.address()
				|| rhs.arguments.len() != 1
			{
				continue;
			}
			let Some(arg) = rhs.arguments[0]
				.as_numeric_literal()
				.and_then(NumericLiteral::as_u32)
				.map(ModuleId::from)
			else {
				continue;
			};

			return Some(arg);
		}
		None
	}
	/// Checks if this module re-exports another whole module and not just parts of it
	fn does_re_export_whole_module(&self) -> Option<ModuleId> {
		self.c
			.does_re_export_whole_module
			.get(|| self.does_re_export_whole_module_impl())
	}

	/// Checks if the given token is usable for a find
	fn igt(&self, t: Token) -> bool {
		_ = self;
		match t.kind() {
			TK::Eof | TK::Undetermined => false,
			// Not only do we never handle a file with a hashbang
			// oxc doesnt even emit them despite having a token kind for them.
			TK::HashbangComment => unreachable!(),
			// if an ident is > 4 chars, it's probably not minified
			TK::Ident => t.span().size() > 4,
			_ => true,
		}
	}

	/// Collect all lazy webpack chunk requires in this module
	/// eg `wreq.e("123456")` in
	/// ```js
	/// Promise.all([
	///     n.e("123456"),
	/// ]).then(n.bind(n, 577593));
	/// ```
	fn collect_lazy_chunk_requires(&self) -> Vec<&'ast CallExpression<'ast>> {
		let Some(wreq) = self.wreq() else {
			return Vec::new();
		};
		let mut calls = Vec::new();
		for n in self.ref_nodes(wreq) {
			let Some(sme) = self
				.p(n.node_id())
				.as_static_member_expression()
			else {
				continue;
			};
			if sme.property.name != "e" {
				continue;
			}
			let Some(call) = self
				.p(sme.node_id())
				.as_call_expression()
			else {
				continue;
			};
			let [chunk_num_str] = call.arguments.as_slice() else {
				continue;
			};
			let Some(_) = chunk_num_str.as_string_literal() else {
				continue;
			};
			calls.push(call);
		}
		calls
	}

	/// returns the spans of all top-level webpack import statements in the main function of this module
	///
	/// simalar to [`Self::count_num_concatentated_modules`]
	fn get_import_spans(&self) -> RangeSet<u32> {
		let mut rs = RangeSet::new();
		let Some(mf) = self.get_main_func() else {
			return rs;
		};
		let mut last_was_import_block = false;
		for stmt in &mf.body.as_ref().unwrap().statements {
			if let Statement::VariableDeclaration(decl) = stmt
				&& self.is_import_decl(decl)
			{
				rs.insert(span_to_range(decl.span()));
				last_was_import_block = true;
			} else if last_was_import_block
				&& let Statement::ExpressionStatement(es) = stmt
				&& self.is_side_effect_import_stmt(es)
			{
				rs.insert(span_to_range(es.span()));
			} else {
				last_was_import_block = false;
			}
		}
		rs
	}

	/// if true, this `seq` would make for a bad sequence of finds
	fn is_good_find_seq(seq: &[Token]) -> bool {
		if seq.len() == 1 {
			!matches!(
				seq[0].kind(),
				TK::Dot
					| TK::Comma | TK::Colon
					| TK::Eq | TK::Eq2
					| TK::Eq3 | TK::Amp2
					| TK::LParen | TK::RParen
					| TK::Pipe2 | TK::Semicolon
					| TK::Question | TK::Extends
					| TK::Let
			)
		} else {
			true
		}
	}

	const MIN_FIND_SCORE: u32 = 8;

	fn impl_generate_finds(&self) -> Vec<ScoredFindSequence> {
		let mut seqs = Vec::new();
		let mut cs = Vec::new();
		let mut bad_spans = self.get_import_spans();
		for chunk_req in self.collect_lazy_chunk_requires() {
			bad_spans.insert(span_to_range(chunk_req.span()));
		}
		for t in self.toks.iter().copied() {
			if self.igt(t) && !bad_spans.contains(&t.start()) {
				cs.push(t);
			} else if !cs.is_empty() {
				self.flush_find_seq(&mut cs, &mut seqs);
			}
		}
		// flush a trailing good run that ends at EOF (no non-good token
		// follows it to trigger the flush inside the loop)
		if !cs.is_empty() {
			self.flush_find_seq(&mut cs, &mut seqs);
		}
		seqs
	}

	/// Scores the accumulated candidate run `cs`, pushing it to `seqs` when it
	/// is a good sequence meeting [`Self::MIN_FIND_SCORE`]. Always leaves `cs`
	/// empty.
	fn flush_find_seq(
		&self,
		cs: &mut Vec<Token>,
		seqs: &mut Vec<ScoredFindSequence>,
	) {
		if Self::is_good_find_seq(cs) {
			let seq = self.score_find_seq(mem::take(cs));
			if seq.score >= Self::MIN_FIND_SCORE {
				seqs.push(seq);
			}
		} else {
			cs.clear();
		}
	}

	#[expect(clippy::too_many_lines)]
	fn score_find_seq(&self, seq: Vec<Token>) -> ScoredFindSequence {
		// start with a base of the find length
		debug_assert!(!seq.is_empty(), "cannot score empty find sequence");
		let mut ts = seq.last().unwrap().span().end - seq[0].span().start;
		let mut intl_keys = Vec::new();
		for t in &seq {
			let tl = t.span().size();
			debug_assert_ne!(tl, 0, "token has zero length");
			if self.is_intl_format_arg(t.span().start)
				&& let Some((_, hashed)) = self.get_i18n_key_at(t.span().start)
			{
				let unhashed = crate::intl::resolve_unhashed_key(&hashed);
				intl_keys.push(IntlKey { hashed, unhashed });
			}
			let score = match t.kind() {
				TK::Ident
				// special-case ident
				| TK::This
				// contextual keywords in ts, idents elsewhere
				| TK::Is // declare function f(a): a is {}
				| TK::String
				| TK::Type
				| TK::Default
				| TK::Object
				// contextual keyword, used as a method/property name
				| TK::Constructor
				| TK::Super
				// strings
				| TK::Str
				| TK::RegExp
				| TK::TemplateHead
				| TK::TemplateMiddle
				| TK::TemplateTail
				| TK::NoSubstitutionTemplate => tl.ilog2() * tl,
				// TODO: handle `0` literal case
				TK::Decimal
				| TK::Float
				| TK::PositiveExponential
				| TK::NegativeExponential => (tl.ilog2() * tl).min(1),
				TK::Comma
				| TK::LParen
				| TK::RParen
				| TK::LCurly
				| TK::RCurly
				| TK::Arrow
				| TK::Dot
				| TK::LBrack
				| TK::RBrack
				| TK::Semicolon => 0,
				TK::Eq
				| TK::Question
				| TK::Question2
				| TK::QuestionDot
				| TK::Colon
				| TK::Eq2
				| TK::Eq3
				| TK::RAngle
				| TK::LAngle
				| TK::Of
				| TK::Try
				| TK::Catch
				| TK::Class
				| TK::Static
				| TK::Async
				| TK::Await
				| TK::Finally
				| TK::Switch
				| TK::Case
				| TK::In
				| TK::GtEq
				| TK::LtEq
				| TK::Amp
				| TK::If
				| TK::Break
				| TK::Var
				| TK::Else
				| TK::Neq
				| TK::Minus2
				| TK::Plus2
				| TK::Plus
				| TK::Neq2
				| TK::Amp2
				| TK::Pipe2
				| TK::Dot3
				| TK::Minus
				| TK::Star
				| TK::Slash
				| TK::Let
				| TK::Bang => 1,
				TK::Null => {
					debug_assert_eq!(tl, 4, "null token should be 4 bytes");
					tl
				}
				TK::Void => {
					debug_assert_eq!(tl, 4, "void token should be 4 bytes");
					tl
				}
				TK::Typeof => {
					debug_assert_eq!(tl, 6, "typeof token should be 6 bytes");
					tl
				}
				TK::From => {
					debug_assert_eq!(tl, 4, "from token should be 4 bytes");
					tl
				}
				TK::Return => {
					debug_assert_eq!(tl, 6, "return token should be 6 bytes");
					tl / 2
				}
				TK::For => {
					debug_assert_eq!(tl, 3, "for token should be 3 bytes");
					tl
				}
				TK::New => {
					debug_assert_eq!(tl, 3, "new token should be 3 bytes");
					tl
				}
				// `new.target`
				TK::Target => {
					debug_assert_eq!(tl, 6, "target token should be 6 bytes");
					tl
				}
				TK::Function => {
					debug_assert_eq!(tl, 8, "function token should be 8 bytes");
					tl / 2
				}
				TK::Instanceof => {
					debug_assert_eq!(
						tl, 10,
						"instanceof token should be 10 bytes"
					);
					tl
				}
				_ => todo!("mid: {:?} score token {:#?}", self.get_module_id(), t),
			};
			ts += score;
		}
		ScoredFindSequence {
			score: ts,
			tokens: seq,
			intl_keys,
		}
	}
}

/// functions to make the raw export map
#[expect(clippy::multiple_inherent_impl)]
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
		ret
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
		// `Object.freeze({...})` — descend into the wrapped object literal
		if let Some(frozen) = self.unwrap_object_freeze(node) {
			return self
				.raw_make_export_map_object_expression(frozen)
				.into();
		}
		RawExportRange::from_node(node).into()
	}
	/// If `node` is a call to `Object.freeze(objectLiteral)`, return the
	/// wrapped object literal.
	fn unwrap_object_freeze(
		&self,
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
						module_id=?self.get_module_id(),
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

#[cfg(test)]
mod tests;
