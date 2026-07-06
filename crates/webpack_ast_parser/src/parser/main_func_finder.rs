use ast_parser::{AstParser, exts::StatementExt as _};
use oxc::{
	ast::ast::{Expression, Function, Statement},
	ast_visit::Visit,
	semantic::{NodeId, ScopeFlags},
};

use std::debug_assert_matches;

use crate::WebpackAstParser;

pub fn find<'ast>(
	parser: &WebpackAstParser<'ast>,
) -> Option<&'ast Function<'ast>> {
	let mut finder = Finder::new();
	finder.visit_program(parser.prog);
	let ret = parser
		.n(finder.ret_val?)
		.kind()
		.as_function()
		.unwrap();
	Some(ret)
}

struct Finder {
	/// Due to lifetime issues we cant store the actual node ref
	/// so we store the node is instead and retrieve the node from the program later
	ret_val: Option<NodeId>,
}

impl Finder {
	const fn new() -> Self {
		Self { ret_val: None }
	}
}

impl Visit<'_> for Finder {
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
		debug_assert_matches!(
			self.ret_val,
			None,
			"Found multiple top-level functions in the program"
		);
		self.ret_val = Some(it.node_id());
	}
}
