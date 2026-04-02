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

mod arg_finder {
	use super::WebpackAstParser;
	use ast_parser::exts::{BindingPatternExt, StatementExt};
	use oxc::{
		ast::ast::{Expression, Function, Statement},
		ast_visit::Visit,
		semantic::{ScopeFlags, SymbolId},
	};

	pub fn find(p: &WebpackAstParser<'_>, arg_index: u8) -> Option<SymbolId> {
		let prog = p.prog;
		let mut finder = ArgFinder {
			param_index: arg_index,
			r: None,
		};
		finder.visit_program(prog);
		finder.r
	}

	struct ArgFinder {
		param_index: u8,
		r: Option<SymbolId>,
	}

	impl Visit<'_> for ArgFinder {
		fn visit_statement(&mut self, it: &Statement) {
			if let Some(expr_stmt) = it.as_expression_statement() {
				self.visit_expression_statement(expr_stmt);
			}
		}
		fn visit_expression(&mut self, it: &Expression) {
			match it {
				Expression::BinaryExpression(e) => {
					self.visit_binary_expression(e);
				}
				Expression::FunctionExpression(e) => {
					self.visit_function(e, ScopeFlags::Function);
				}
				_ => {}
			}
		}
		fn visit_function(&mut self, it: &Function, _: ScopeFlags) {
			let params = &it.params.items;
			let num_params = params.len();
			if it.params.rest.is_some()
				|| num_params > 3
				|| num_params <= self.param_index as usize + 1
			{
				return;
			}
			let Some(ident) = params[self.param_index as usize]
				.pattern
				.as_binding_identifier()
			else {
				return;
			};
			debug_assert!(self.r.is_none(), "multiple functions found");
			self.r = Some(ident.symbol_id());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use insta::assert_ron_snapshot;
	use itertools::Itertools;

	macro_rules! parse {
		($alloc:expr, $source:literal) => {{
			let source = include_str!($source);
			WebpackAstParser::try_new(&$alloc, source).unwrap()
		}};
	}

	#[test]
	fn constructs() {
		let alloc = Allocator::new();
		let source = include_str!("test_data/wp/module.js");
		_ = WebpackAstParser::try_new(&alloc, source).unwrap();
	}
}
