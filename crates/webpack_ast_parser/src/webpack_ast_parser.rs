use anyhow::Result;
use ast_parser::{
	AstParser,
	exts::{BindingPatternExt, ExpressionExt, NumericLiteralExt as _},
	parse,
};
use oxc::{
	allocator::Allocator,
	ast::{
		AstKind,
		ast::{NumericLiteral, Program},
	},
	semantic::{AstNode, Semantic, SymbolId},
	span::SourceType,
};

use crate::{
	bundle::{DefaultModuleCache, IModuleCache},
	cache::CacheValue,
	types::ModuleId,
};

pub struct WebpackAstParser<'ast> {
	prog: &'ast Program<'ast>,
	sema: Semantic<'ast>,
	source: &'ast str,
	module_cache: &'ast dyn IModuleCache<'ast>,
	/// Internal cache
	c: Cache,
}

#[derive(Default)]
struct Cache {
	wreq: CacheValue<Option<SymbolId>>,
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

			debug_assert!(self.source[start..end]
				.chars()
				.all(|c| c.is_ascii_digit()));
			let id = self.source[start..end].parse().ok()?;

			return Some(ModuleId(id));
		}
		None
	}
}

// Private API
#[allow(clippy::multiple_inherent_impl)]
impl<'ast> WebpackAstParser<'ast> {
	fn wreq(&self) -> Option<SymbolId> {
		self.c
			.wreq
			.get(|| self.find_webpack_arg(2))
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
}

mod arg_finder;

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
mod tests {
	use super::*;
	use insta::{assert_debug_snapshot, assert_ron_snapshot};
	use itertools::Itertools;
	use oxc::span::{Atom, GetSpan as _, Span};

	macro_rules! parse {
		($alloc:expr, $source:literal) => {{
			let source = include_str!($source);
			WebpackAstParser::try_new(&$alloc, source).unwrap()
		}};
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
				todo!()
			}
			#[test]
			fn string_literal_export() {
				todo!()
			}

			#[test]
			fn object_literal_export() {
				todo!()
			}

			#[test]
			fn object_with_computed_prop() {
				todo!()
			}

			#[test]
			fn class_export() {
				todo!()
			}

			#[test]
			fn enum_export() {
				todo!()
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
