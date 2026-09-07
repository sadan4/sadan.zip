#![allow(clippy::unreadable_literal, reason = "we want verbatim module ids")]
use crate::{
	JsHashEntry,
	Sealed,
	base::{WebpackChunkParser, WebpackChunkParserImpl},
};
use ast_parser::{
	AstParser,
	NodeLocationIndex,
	cache,
	exts::{
		BindingPatternExt,
		ExpressionExt,
		MemberExpressionExt,
		NumericLiteralExt,
		StatementExt as _,
	},
	parse,
};
use explorer_types::ModuleId;
use memchr::memmem::Finder;
use oxc::{
	allocator::Allocator,
	ast::ast::{
		ArrowFunctionExpression,
		BinaryOperator,
		Expression,
		ObjectExpression,
		ObjectProperty,
		Program,
		PropertyKind,
		VariableDeclarator,
	},
	semantic::{ReferenceFlags, Semantic, SymbolFlags, SymbolId},
	span::SourceType,
};
use parser_diag::{PResult, err, err_ns};
use regex::Regex;
use smol_str::{SmolStr, SmolStrBuilder, ToSmolStr};
use std::sync::LazyLock;

// FIXME: add basic caching with OnceCell
pub struct WebpackMainChunkParser<'ast> {
	source_text: &'ast str,
	prog: &'ast Program<'ast>,
	sema: Semantic<'ast>,
	node_index: cache::Ref<NodeLocationIndex<'ast>>,
}

const WEBPACK_EXPORTS_NAME: &str = "__webpack_exports__";
const KNOWN_BUILD_MODULE_IDS: &[ModuleId] =
	&[ModuleId(128014), ModuleId(446023), ModuleId(927815)];
static BUILD_MODULE_NEEDLE: LazyLock<Finder<'static>> = LazyLock::new(|| {
	Finder::new(b"Trying to open a changelog for an invalid build number")
});
static BUILD_NUMBER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r#"(?:parseInt\("|"Trying to open a changelog for an invalid build number )(\d+?)"\)"#).unwrap()
});

fn as_valid_module_id<'ast>(expr: &'ast Expression<'ast>) -> Option<ModuleId> {
	match expr {
		Expression::NumericLiteral(n) => n.as_u32().map(ModuleId),
		Expression::StringLiteral(s) => s.value.parse().ok().map(ModuleId),
		_ => None,
	}
}

fn handle_chunk_cond_rhs(
	module_id: &str,
	rhs: &Expression,
) -> PResult<JsHashEntry> {
	match rhs {
		Expression::BinaryExpression(cur) => {
			let cur = cur.as_ref();
			let chunk_hash_with_ext = &*cur
				.right
				.as_string_literal()
				.ok_or_else(|| {
					err(&cur.right, "chunk hash is not a string literal")
				})?
				.value;
			let chunk_hash = chunk_hash_with_ext
				.strip_suffix(".js")
				.unwrap_or(chunk_hash_with_ext);
			let mut sb = SmolStrBuilder::new();
			sb.push_str(module_id);
			sb.push_str(chunk_hash);
			Ok(JsHashEntry {
				chunk_id: module_id.to_smolstr(),
				hash: sb.finish(),
			})
		}
		// if the module id is small enough, it might be inlined instead of concatenated
		Expression::StringLiteral(cur) => {
			let cur = &*cur.as_ref().value;
			let chunk_hash = cur.strip_suffix(".js").unwrap_or(cur);
			Ok(JsHashEntry {
				chunk_id: module_id.to_smolstr(),
				hash: chunk_hash.to_smolstr(),
			})
		}
		_ => Err(err(
			rhs,
			"chunk hash is neither a concatenation nor a string literal",
		)),
	}
}

// TODO: cache get_webpack_require
impl<'ast> WebpackMainChunkParser<'ast> {
	pub fn try_new(
		alloc: &'ast Allocator,
		source_text: &'ast str,
	) -> PResult<Self> {
		let (prog, sema) = parse(alloc, source_text, SourceType::script())
			.map_err(|e| err_ns("Failed to parse the main chunk").s(e))?;
		Ok(Self {
			source_text,
			prog,
			sema,
			node_index: cache::Ref::new(),
		})
	}
	/// gets `__webpack_require__`
	fn get_webpack_require(&self) -> PResult<SymbolId> {
		let root_iife_scope_id = self.root_iife_scope_id()?;
		let scoping = self.sema.scoping();
		for sym_id in scoping.iter_bindings_in(root_iife_scope_id) {
			if !scoping
				.symbol_flags(sym_id)
				.contains(SymbolFlags::Function)
			{
				continue;
			}
			if self.prop_set(sym_id, "nmd") && self.prop_set(sym_id, "hmd") {
				return Ok(sym_id);
			}
		}
		Err(err_ns("Failed to find webpack require function"))
	}

	fn root_iife_scope_id(&self) -> PResult<oxc::semantic::ScopeId> {
		Ok(self.root_iife()?.scope_id())
	}

	/// the arrow function the whole chunk is wrapped in
	fn root_iife(&self) -> PResult<&'ast ArrowFunctionExpression<'ast>> {
		let first_stmt = self.prog.body.first().ok_or_else(|| {
			err(self.prog, "program has no top-level statements")
		})?;
		let top_level_expr = &first_stmt
			.as_expression_statement()
			.ok_or_else(|| {
				err(
					first_stmt,
					"first top-level statement is not an expression statement",
				)
			})?
			.expression;
		let call = top_level_expr
			.as_call_expression()
			.ok_or_else(|| {
				err(
					top_level_expr,
					"first top-level statement is not a call expression",
				)
			})?;
		let callee = call.callee.get_inner_expression();
		callee
			.as_arrow_function_expression()
			.ok_or_else(|| {
				err(callee, "root IIFE callee is not an arrow function")
			})
	}

	fn prop_set(&self, obj: SymbolId, prop_name: impl AsRef<str>) -> bool {
		for ref_ in self
			.sema
			.scoping()
			.get_resolved_references(obj)
		{
			if !ref_
				.flags()
				.contains(ReferenceFlags::MemberWriteTarget)
			{
				continue;
			}
			let Some(member_write_expr) = self
				.p(ref_.node_id())
				.as_static_member_expression()
			else {
				continue;
			};
			if member_write_expr.property.name == prop_name.as_ref() {
				return true;
			}
		}
		false
	}
	/// gets `__webpack_modules__`
	fn get_webpack_modules(&self) -> PResult<SymbolId> {
		let root_iife_scope_id = self.root_iife_scope_id()?;
		let mut cur = Option::<&VariableDeclarator>::None;
		for sym_id in self
			.sema
			.scoping()
			.iter_bindings_in(root_iife_scope_id)
		{
			let _: Option<()> = try {
				let decl_id = self
					.sema
					.scoping()
					.symbol_declaration(sym_id);
				let decl_parent = self.n(decl_id).kind();
				let decl_parent = decl_parent.as_variable_declarator()?;
				let init = decl_parent.init.as_ref()?;
				let init = init.as_object_expression()?;
				let Some(cur_decl) = cur else {
					cur = Some(decl_parent);
					continue;
				};
				if init.properties.len()
					> cur_decl
						.init
						.as_ref()
						.unwrap()
						.as_object_expression()
						.unwrap()
						.properties
						.len()
				{
					cur = Some(decl_parent);
				}
			};
		}
		cur.map(|decl| {
			decl.id
				.as_binding_identifier()
				.unwrap()
				.symbol_id()
		})
		.ok_or_else(|| err_ns("Failed to find webpack modules object"))
	}

	fn parse_js_hash_map_entry(prop: &ObjectProperty) -> PResult<JsHashEntry> {
		if prop.method
			|| prop.shorthand
			|| prop.computed
			|| prop.kind != PropertyKind::Init
		{
			return Err(err(
				prop,
				"chunk hash map entry is not a plain key/value property",
			));
		}
		let id = prop
			.key
			.try_parse_string_or_number_literal()
			.ok_or_else(|| {
				err(&prop.key, "chunk id is not a string or number literal")
			})?;
		let hash = prop
			.value
			.as_string_literal_like()
			.ok_or_else(|| {
				err(&prop.value, "chunk hash is not a string literal")
			})?;
		let ret = JsHashEntry {
			chunk_id: id.into(),
			hash: hash.as_str().into(),
		};
		Ok(ret)
	}

	fn process_wreq_u_map_expr(
		expr: &'ast Expression<'ast>,
	) -> PResult<Vec<JsHashEntry>> {
		let bin_expr = expr
			.as_binary_expression()
			.ok_or_else(|| {
				err(expr, "`wreq.u` body is not a binary expression")
			})?;
		if bin_expr.operator != BinaryOperator::Addition {
			return Err(err(bin_expr, "`wreq.u` body is not a concatenation"));
		}
		let concat_with_hash_map = bin_expr
			.left
			.as_binary_expression()
			.ok_or_else(|| {
				err(
					&bin_expr.left,
					"`wreq.u` body lhs is not a binary expression",
				)
			})?;
		if concat_with_hash_map.operator != BinaryOperator::Addition {
			return Err(err(
				concat_with_hash_map,
				"`wreq.u` body lhs is not a concatenation",
			));
		}
		let member = concat_with_hash_map
			.right
			.as_computed_member()
			.ok_or_else(|| {
				err(
					&concat_with_hash_map.right,
					"chunk hash map is not indexed with a computed member expression",
				)
			})?;
		let hash_map_expr = member.object.get_inner_expression();
		let hash_map = hash_map_expr
			.as_object_expression()
			.ok_or_else(|| {
				err(hash_map_expr, "chunk hash map is not an object expression")
			})?;
		let mut ret = Vec::with_capacity(hash_map.properties.len());
		for prop in &hash_map.properties {
			let prop = prop.as_property().ok_or_else(|| {
				err(prop, "chunk hash map entry is not an object property")
			})?;
			ret.push(Self::parse_js_hash_map_entry(prop)?);
		}
		Ok(ret)
	}

	pub fn get_js_chunk_hashes(&self) -> PResult<Vec<JsHashEntry>> {
		let wreq = self.get_webpack_require()?;
		let uses = self
			.sema
			.scoping()
			.get_resolved_references(wreq);
		let u_func = 'u: {
			for u in uses {
				let _: Option<()> = try {
					let parent = self
						.p(u.node_id())
						.as_static_member_expression()?;
					if parent.property.name != "u" {
						continue;
					}
					let assign = self
						.p(parent.node_id())
						.as_assignment_expression()?;
					let func = assign
						.right
						.as_arrow_function_expression()?;
					break 'u func;
				};
			}
			return Err(err_ns("Failed to find `wreq.u`"));
		};
		// expect body to be BinExp>[BinExp>["" + {id:hash}[id]] + ".js"]
		let mut cur = u_func.get_expression().ok_or_else(|| {
			err(u_func, "`wreq.u` body is not a single expression")
		})?;
		let mut ret = Vec::new();
		while let Some(expr) = cur.as_conditional_expression() {
			let test = expr.test.get_inner_expression();
			let cond = test
				.as_binary_expression()
				.ok_or_else(|| {
					err(test, "chunk id test is not a binary expression")
				})?;
			if cond.operator != BinaryOperator::StrictEquality {
				return Err(err(
					cond,
					"chunk id test is not a strict equality",
				));
			}
			let chunk_id = cond
				.left
				.as_string_literal()
				.map(|s| &*s.value)
				.or_else(|| {
					cond.right
						.as_string_literal()
						.map(|s| &*s.value)
				})
				.ok_or_else(|| {
					err(
						cond,
						"neither side of the chunk id test is a string literal",
					)
				})?;
			let rhs = &expr.consequent;
			let entry = handle_chunk_cond_rhs(chunk_id, rhs)?;
			ret.push(entry);
			cur = &expr.alternate;
		}
		let from_map_expr = Self::process_wreq_u_map_expr(cur)?;
		ret.extend(from_map_expr);
		Ok(ret)
	}

	fn get_entrypoint_id_1(&self) -> PResult<ModuleId> {
		let wreq_sym_id = self.get_webpack_require()?;
		let uses = self
			.sema
			.scoping()
			.get_resolved_references(wreq_sym_id);
		for u in uses {
			let _: Option<()> = try {
				let call = self
					.p(u.node_id())
					.as_call_expression()?;

				if call.arguments.len() != 1 {
					continue;
				}

				let maybe_id = call.arguments[0]
					.as_expression()
					.and_then(as_valid_module_id)?;

				let decl = self
					.p(call.node_id())
					.as_variable_declarator()?;

				if decl
					.id
					.as_binding_identifier()
					.is_none_or(|ident| ident.name != WEBPACK_EXPORTS_NAME)
				{
					continue;
				}

				return Ok(maybe_id);
			};
		}
		Err(err_ns(
			"Failed to find a `__webpack_exports__` entrypoint call",
		))
	}

	fn get_entrypoint_id_2(&self) -> PResult<ModuleId> {
		let af = &self.root_iife()?.body;
		let last_expr = if let Some(expr) = af.as_expression() {
			expr
		} else {
			let body = af.as_function_body().ok_or_else(|| {
				err(
					af,
					"root IIFE body is neither an expression nor a function body",
				)
			})?;
			let last_stmt = body
				.statements
				.last()
				.ok_or_else(|| err(body, "root IIFE body is empty"))?;
			&last_stmt
				.as_expression_statement()
				.ok_or_else(|| {
					err(
						last_stmt,
						"last statement of the root IIFE is not an expression statement",
					)
				})?
				.expression
		};
		let last_expr = last_expr.get_inner_expression();
		let seq = last_expr
			.as_sequence_expression()
			.ok_or_else(|| {
				err(last_expr, "root IIFE tail is not a sequence expression")
			})?;
		let last_in_seq = seq.expressions.last().ok_or_else(|| {
			err(seq, "root IIFE tail sequence expression is empty")
		})?;
		let entry_call = last_in_seq
			.as_call_expression()
			.ok_or_else(|| {
				err(last_in_seq, "entrypoint is not a call expression")
			})?;
		let callee = entry_call
			.callee
			.as_identifier()
			.ok_or_else(|| {
				err(
					&entry_call.callee,
					"entrypoint callee is not an identifier",
				)
			})?;
		if !self.cmp_sym(callee, &self.get_webpack_require()?) {
			return Err(err(
				callee,
				"entrypoint callee is not `__webpack_require__`",
			));
		}
		if entry_call.arguments.len() != 1 {
			return Err(err(
				entry_call,
				"expected the entrypoint call to have exactly one argument",
			));
		}
		let arg = &entry_call.arguments[0];
		arg.as_numeric_literal()
			.ok_or_else(|| err(arg, "entrypoint id is not a numeric literal"))?
			.as_u32()
			.map(ModuleId)
			.ok_or_else(|| err(arg, "entrypoint id does not fit in a u32"))
	}

	fn get_entrypoint_id_3(&self) -> PResult<ModuleId> {
		let wreq = self.get_webpack_require()?;
		let call_expr = 'c: {
			for node in self.refs(wreq) {
				let _: Option<()> = try {
					let mem_expr = self
						.p(node)
						.as_static_member_expression()?;
					if mem_expr.property.name != "O" {
						continue;
					}
					let call = self
						.p(mem_expr.node_id())
						.as_call_expression()?;
					if call.arguments.len() != 3
						|| self
							.p(call.node_id())
							.as_variable_declarator()
							.is_none()
					{
						continue;
					}
					break 'c call;
				};
			}
			return Err(err_ns("Failed to find a `wreq.O` entrypoint call"));
		};
		let cb = &call_expr.arguments[2];
		let cb = cb
			.as_arrow_function_expression()
			.ok_or_else(|| {
				err(cb, "`wreq.O` callback is not an arrow function")
			})?;
		let cb_body = cb.get_expression().ok_or_else(|| {
			err(cb, "`wreq.O` callback body is not a single expression")
		})?;
		let entry_call = cb_body
			.as_call_expression()
			.ok_or_else(|| {
				err(cb_body, "`wreq.O` callback body is not a call expression")
			})?;
		let arg = entry_call
			.arguments
			.first()
			.ok_or_else(|| {
				err(entry_call, "entrypoint call has no arguments")
			})?;
		arg.as_numeric_literal()
			.ok_or_else(|| err(arg, "entrypoint id is not a numeric literal"))?
			.as_u32()
			.map(ModuleId)
			.ok_or_else(|| err(arg, "entrypoint id does not fit in a u32"))
	}

	pub fn get_entrypoint_id(&self) -> PResult<ModuleId> {
		self.get_entrypoint_id_1()
			.or_else(|e1| {
				self.get_entrypoint_id_2()
					.map_err(|e2| e2.s(e1))
			})
			.or_else(|e2| {
				self.get_entrypoint_id_3()
					.map_err(|e3| e3.s(e2))
			})
	}

	pub fn get_build_number(&self) -> PResult<SmolStr> {
		let modules = self.get_defined_modules()?;
		// use known build modules to save time
		// TODO: perform a manual search if this fails
		// FIXME: warn when using BUILD_MODULE_NEEDLE
		for maybe_known_id in KNOWN_BUILD_MODULE_IDS {
			if let Some(m_txt) = modules.get(maybe_known_id)
				&& BUILD_MODULE_NEEDLE
					.find(m_txt.as_bytes())
					.is_some()
			{
				let id = BUILD_NUMBER_REGEX
					.captures(m_txt)
					.and_then(|caps| caps.get(1))
					.ok_or_else(|| {
						err_ns(
							"Failed to find the build number in the build module",
						)
					})?
					.as_str()
					.into();
				return Ok(id);
			}
		}
		Err(err_ns("Failed to find the build module"))
	}
}

impl<'ast> AstParser<'ast> for WebpackMainChunkParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}

	fn node_location_index(&self) -> &cache::Ref<NodeLocationIndex<'ast>> {
		&self.node_index
	}
}

impl Sealed for WebpackMainChunkParser<'_> {}

impl<'ast> WebpackChunkParserImpl<'ast> for WebpackMainChunkParser<'ast> {
	fn get_module_object(&self) -> PResult<&'ast ObjectExpression<'ast>> {
		let wp_modules_sym_id = self.get_webpack_modules()?;
		let decl = self
			.sema
			.symbol_declaration(wp_modules_sym_id)
			.kind()
			.as_variable_declarator()
			.ok_or_else(|| {
				err_ns(
					"webpack modules object is not declared by a variable declarator",
				)
			})?;
		let init = decl.init.as_ref().ok_or_else(|| {
			err(decl, "webpack modules declaration has no initializer")
		})?;
		init.as_object_expression()
			.ok_or_else(|| {
				err(init, "webpack modules is not an object expression")
			})
	}

	fn get_source_text(&self) -> &'ast str {
		self.source_text
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use insta::assert_ron_snapshot;
	use itertools::Itertools;
	use oxc::allocator::Allocator;

	macro_rules! parse {
		($alloc:expr, $source:literal) => {{
			let source = include_str!($source);
			WebpackMainChunkParser::try_new(&$alloc, source).unwrap()
		}};
	}

	// old format
	#[test]
	fn format_1() {
		let alloc = Allocator::new();
		let parser = parse!(alloc, "test_data/fullWeb.js");
		{
			let entrypoint = parser.get_entrypoint_id().unwrap();
			assert_eq!(entrypoint, ModuleId(650204));
		};
		{
			let build_number = parser.get_build_number().unwrap();
			assert_eq!(build_number, "440786");
		};
		{
			let mut hashes = parser.get_js_chunk_hashes().unwrap();
			hashes.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
			assert_ron_snapshot!(hashes);
		};
		{
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_keys()
				.sorted()
				.collect_vec();
			assert_ron_snapshot!(modules);
		};
	}
	// new format
	#[test]
	fn format_2() {
		let alloc = Allocator::new();
		let parser = parse!(alloc, "test_data/fullWeb2.js");
		{
			let entrypoint = parser.get_entrypoint_id().unwrap();
			assert_eq!(entrypoint, ModuleId(329563));
		};
		{
			let build_number = parser.get_build_number().unwrap();
			assert_eq!(build_number, "492031");
		};
		{
			let mut hashes = parser.get_js_chunk_hashes().unwrap();
			hashes.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
			assert_ron_snapshot!(hashes);
		};
		{
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_keys()
				.sorted()
				.collect_vec();
			assert_ron_snapshot!(modules);
		};
	}

	#[test]
	fn format_3() {
		let alloc = Allocator::new();
		let parser = parse!(alloc, "test_data/fullWeb3.js");
		{
			let entrypoint = parser.get_entrypoint_id().unwrap();
			assert_eq!(entrypoint, ModuleId(329563), "entrypoint mismatch");
		};
		{
			let build_number = parser.get_build_number().unwrap();
			assert_eq!(build_number, "533645", "build number mismatch");
		};
		{
			let mut hashes = parser.get_js_chunk_hashes().unwrap();
			hashes.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
			assert_ron_snapshot!(hashes);
		};
		{
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_keys()
				.sorted()
				.collect_vec();
			assert_ron_snapshot!(modules);
		};
	}

	#[test]
	fn format_4() {
		let alloc = Allocator::new();
		let parser = parse!(alloc, "test_data/fullWeb4.js");
		{
			let entrypoint = parser.get_entrypoint_id().unwrap();
			assert_eq!(entrypoint, ModuleId(329563), "entrypoint mismatch");
		};
		{
			let build_number = parser.get_build_number().unwrap();
			assert_eq!(build_number, "538030", "build number mismatch");
		};
		{
			let mut hashes = parser.get_js_chunk_hashes().unwrap();
			hashes.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
			assert_ron_snapshot!(hashes);
		};
		{
			let modules = parser
				.get_defined_modules()
				.unwrap()
				.into_keys()
				.sorted()
				.collect_vec();
			assert_ron_snapshot!(modules);
		};
	}
}
