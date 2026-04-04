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
		ret_val: None,
	};
	finder.visit_program(prog);
	finder.ret_val
}

struct ArgFinder {
	param_index: u8,
	ret_val: Option<SymbolId>,
}

impl Visit<'_> for ArgFinder {
	fn visit_statement(&mut self, it: &Statement) {
		if let Some(expr_stmt) = it.as_expression_statement() {
			self.visit_expression_statement(expr_stmt);
		}
	}
	fn visit_expression(&mut self, it: &Expression) {
		match it {
			Expression::SequenceExpression(e) => {
				self.visit_sequence_expression(e);
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
			|| num_params < self.param_index as usize + 1
		{
			return;
		}
		let Some(ident) = params[self.param_index as usize]
			.pattern
			.as_binding_identifier()
		else {
			return;
		};
		debug_assert!(self.ret_val.is_none(), "multiple functions found");
		self.ret_val = Some(ident.symbol_id());
	}
}
