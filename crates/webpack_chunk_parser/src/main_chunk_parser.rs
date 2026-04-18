#![allow(clippy::unreadable_literal, reason = "we want verbatim module ids")]
use crate::{
	JsHashEntry,
	Sealed,
	base::{WebpackChunkParser, WebpackChunkParserImpl},
};
use anyhow::{Result, anyhow};
use ast_parser::{
	AstParser,
	exts::{
		BindingPatternExt,
		ExpressionExt,
		MemberExpressionExt,
		NumericLiteralExt,
	},
	parse,
};
use explorer_types::ModuleId;
use memchr::memmem::Finder;
use oxc::{
	allocator::Allocator,
	ast::ast::{
		BinaryOperator,
		Expression,
		ObjectExpression,
		ObjectProperty,
		Program,
		PropertyKind,
	},
	semantic::{Semantic, SymbolId},
	span::SourceType,
};
use regex::Regex;
use smol_str::SmolStr;
use std::sync::LazyLock;

// FIXME: add basic caching with OnceCell
pub struct WebpackMainChunkParser<'ast> {
	source_text: &'ast str,
	prog: &'ast Program<'ast>,
	sema: Semantic<'ast>,
}

const WEBPACK_REQUIRE_NAME: &str = "__webpack_require__";
const WEBPACK_MODULES_NAME: &str = "__webpack_modules__";
const WEBPACK_EXPORTS_NAME: &str = "__webpack_exports__";
const KNOWN_BUILD_MODULE_IDS: &[ModuleId] =
	&[ModuleId(128014), ModuleId(446023)];
static BUILD_MODULE_NEEDLE: LazyLock<Finder<'static>> = LazyLock::new(|| {
	Finder::new(b"Trying to open a changelog for an invalid build number")
});
static BUILD_NUMBER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r#"(?:parseInt\("|"Trying to open a changelog for an invalid build number )(\d+?)"\)"#).unwrap()
});

fn as_valid_module_id<'ast>(expr: &'ast Expression<'ast>) -> Option<ModuleId> {
	match expr {
		Expression::NumericLiteral(n) => n.as_u32().map(Into::into),
		_ => None,
	}
}

impl<'ast> WebpackMainChunkParser<'ast> {
	pub fn try_new(
		alloc: &'ast Allocator,
		source_text: &'ast str,
	) -> Result<Self> {
		let (prog, sema) = parse(alloc, source_text, SourceType::script())
			.map_err(|e| anyhow!(e))?;
		Ok(Self {
			source_text,
			prog,
			sema,
		})
	}
	/// gets `__webpack_require__`
	fn get_webpack_require(&self) -> Option<SymbolId> {
		// i think this is the most efficient want to do this
		for scope_id in self
			.sema
			.scoping()
			.scope_descendants_from_root()
		{
			if let Some(symbol_id) = self
				.sema
				.scoping()
				.get_binding(scope_id, WEBPACK_REQUIRE_NAME.into())
			{
				return Some(symbol_id);
			}
		}
		None
	}
	/// gets `__webpack_modules__`
	fn get_webpack_modules(&self) -> Option<SymbolId> {
		for scope_id in self
			.sema
			.scoping()
			.scope_descendants_from_root()
		{
			if let Some(symbol_id) = self
				.sema
				.scoping()
				.get_binding(scope_id, WEBPACK_MODULES_NAME.into())
			{
				return Some(symbol_id);
			}
		}
		None
	}

	fn parse_js_hash_map_entry(prop: &ObjectProperty) -> Option<JsHashEntry> {
		if prop.method
			|| prop.shorthand
			|| prop.computed
			|| prop.kind != PropertyKind::Init
		{
			return None;
		}
		let id = prop
			.key
			.try_parse_string_or_number_literal()?;
		let hash = prop.value.as_string_literal_like()?;
		let ret = JsHashEntry {
			chunk_id: id.into(),
			hash: hash.as_str().into(),
		};
		Some(ret)
	}

	pub fn get_js_chunk_hashes(&self) -> Option<Vec<JsHashEntry>> {
		let wreq = self.get_webpack_require()?;
		let uses = self
			.sema
			.scoping()
			.get_resolved_references(wreq);
		let u_func = 'u: {
			for u in uses {
				let Some(parent) = self
					.p(u.node_id())
					.as_static_member_expression()
				else {
					continue;
				};
				if parent.property.name != "u" {
					continue;
				}
				let Some(assign) = self
					.p(parent.node_id())
					.as_assignment_expression()
				else {
					continue;
				};
				let Some(func) = assign
					.right
					.as_arrow_function_expression()
				else {
					continue;
				};
				break 'u func;
			}
			return None;
		};
		// expect body to be BinExp>[BinExp>["" + {id:hash}[id]] + ".js"]
		let expr = u_func
			.get_expression()?
			.as_binary_expression()?;
		if expr.operator != BinaryOperator::Addition {
			return None;
		}
		let concat_with_hash_map = expr.left.as_binary_expression()?;
		if concat_with_hash_map.operator != BinaryOperator::Addition {
			return None;
		}
		let hash_map = concat_with_hash_map
			.right
			.as_computed_member()?
			.object
			.get_inner_expression()
			.as_object_expression()?;
		let mut ret = Vec::with_capacity(hash_map.properties.len());
		for prop in &hash_map.properties {
			ret.push(Self::parse_js_hash_map_entry(prop.as_property()?)?);
		}
		Some(ret)
	}

	pub fn get_entrypoint_id(&self) -> Option<ModuleId> {
		let wreq_sym_id = self.get_webpack_require()?;
		let uses = self
			.sema
			.scoping()
			.get_resolved_references(wreq_sym_id);
		for u in uses {
			let Some(call) = self.p(u.node_id()).as_call_expression() else {
				continue;
			};

			if call.arguments.len() != 1 {
				continue;
			}

			let Some(maybe_id) = call.arguments[0]
				.as_expression()
				.and_then(as_valid_module_id)
			else {
				continue;
			};

			let Some(decl) = self
				.p(call.node_id())
				.as_variable_declarator()
			else {
				continue;
			};

			if decl
				.id
				.as_binding_identifier()
				.is_none_or(|ident| ident.name != WEBPACK_EXPORTS_NAME)
			{
				continue;
			}

			return Some(maybe_id);
		}
		None
	}

	pub fn get_build_number(&self) -> Option<SmolStr> {
		let modules = self.get_defined_modules()?;
		// use known build modules to save time
		// TODO: perform a manual search if this fails
		for maybe_known_id in KNOWN_BUILD_MODULE_IDS {
			if let Some(m_txt) = modules.get(maybe_known_id)
				&& BUILD_MODULE_NEEDLE
					.find(m_txt.as_bytes())
					.is_some()
			{
				let id = BUILD_NUMBER_REGEX
					.captures(m_txt)?
					.get(1)?
					.as_str()
					.into();
				return Some(id);
			}
		}
		None
	}
}

impl<'ast> AstParser<'ast> for WebpackMainChunkParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}
}

impl Sealed for WebpackMainChunkParser<'_> {}

impl<'ast> WebpackChunkParserImpl<'ast> for WebpackMainChunkParser<'ast> {
	fn get_module_object(&self) -> Option<&'ast ObjectExpression<'ast>> {
		let wp_modules_sym_id = self.get_webpack_modules()?;
		self.sema
			.symbol_declaration(wp_modules_sym_id)
			.kind()
			.as_variable_declarator()?
			.init
			.as_ref()?
			.as_object_expression()
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
			let entrypoint = parser.get_entrypoint_id();
			assert_eq!(entrypoint, Some(ModuleId(650204)));
		};
		{
			let build_number = parser.get_build_number();
			assert_eq!(build_number.as_deref(), Some("440786"));
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
			let entrypoint = parser.get_entrypoint_id();
			assert_eq!(entrypoint, Some(ModuleId(329563)));
		};
		{
			let build_number = parser.get_build_number();
			assert_eq!(build_number.as_deref(), Some("492031"));
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
