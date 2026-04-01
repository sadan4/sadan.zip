mod flatten_template;
mod fold_bin_exp;
mod inline;
mod inline_enums;
mod string_raw;
mod util;

use oxc::{
	allocator::Allocator,
	ast::ast::Program,
	semantic::{Scoping, Semantic, SemanticBuilder},
};
use oxc_traverse::{Traverse, traverse_mut};

pub use flatten_template::FlattenTemplatePass;
pub use fold_bin_exp::FoldBinaryExpressionsPass;
pub use inline::InlineConstantsPass;
pub use inline_enums::InlineEnumsPass;
pub use string_raw::EvalStringRawPass;

pub struct PassManager<'ast> {
	program: &'ast mut Program<'ast>,
	scoping: Scoping,
	alloc: &'ast Allocator,
}

impl<'ast> PassManager<'ast> {
	pub fn new(
		alloc: &'ast Allocator,
		(program, scoping): (&'ast mut Program<'ast>, Scoping),
	) -> Self {
		Self {
			program,
			scoping,
			alloc,
		}
	}
	pub fn run_pass(mut self, mut pass: impl Traverse<'ast, ()>) -> Self {
		let new_scoping =
			traverse_mut(&mut pass, self.alloc, self.program, self.scoping, ());
		self.scoping = new_scoping;
		self
	}
	pub fn finish(self) -> (&'ast Program<'ast>, Semantic<'ast>) {
		let prog = self.program;
		let sema = SemanticBuilder::new()
			.with_cfg(true)
			.with_check_syntax_error(true)
			.build(prog);
		assert!(
			sema.errors.is_empty(),
			"Passes created invalid AST: {:#?}",
			sema.errors
		);
		(prog, sema.semantic)
	}
}

#[cfg(test)]
#[expect(clippy::items_after_test_module, reason = "export testing util macro")]
mod test_util {
	use super::*;
	#[expect(dead_code)]
	pub struct NoopPass;
	impl Traverse<'_, ()> for NoopPass {}
	pub fn dump_ast(parser: &Program<'_>) -> String {
		Codegen::new()
			.with_options(CodegenOptions {
				single_quote: true,
				minify: false,
				comments: CommentOptions {
					annotation: true,
					jsdoc: true,
					legal: LegalComment::Inline,
					normal: true,
				},
				indent_char: IndentChar::Tab,
				indent_width: 1,
				initial_indent: 0,
				source_map_path: None,
			})
			.build(parser)
			.code
	}
	#[macro_export]
	macro_rules! test_pass {
		($code:expr, $pass:expr) => {{
			let alloc = oxc::allocator::Allocator::new();
			let pass_data = $crate::vc::parser::ast_parser::parse_for_traverse(
				&alloc,
				$code,
				::oxc::span::SourceType::tsx(),
			);
			let (prog, _) =
				$crate::vc::parser::vencord_ast_parser::pass::PassManager::new(
					&alloc,
					pass_data.unwrap(),
				)
				.run_pass($pass)
				.finish();
			$crate::vc::parser::vencord_ast_parser::pass::dump_ast(prog)
		}};
	}
	use oxc::codegen::{
		Codegen,
		CodegenOptions,
		CommentOptions,
		IndentChar,
		LegalComment,
	};
}

#[cfg(test)]
pub use test_util::dump_ast;
