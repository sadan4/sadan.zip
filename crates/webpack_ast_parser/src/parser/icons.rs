use ast_parser::AstParser as _;
use oxc::ast::{AstKind, ast::Statement};

use crate::{
	WebpackAstParser,
	export_map::{ExportRange, ExportValue},
};

impl<'ast> WebpackAstParser<'ast> {
	pub fn is_default_icon_props_module(&self) -> bool {
		let raw_map = self.get_export_map_raw();
		if raw_map.exports.len() != 1 {
			return false;
		}
		let (k, ExportValue::Range(ExportRange(v, _))) =
			raw_map.exports.iter().next().unwrap()
		else {
			return false;
		};
		let Some(func) = v.last() else {
			return false;
		};
		let Some(function) =
			self.find_parent(func.node_id(), AstKind::as_function)
		else {
			return false;
		};
        // function _(e) {
		//     let t = null != e["aria-label"];
		//     return e["aria-hidden"] = e["aria-hidden"] ?? !t,
		//     e.role = e.role ?? "img",
		//     e
        // }
		let [
			Statement::VariableDeclaration(decl),
			Statement::ReturnStatement(ret),
		] = function
			.body
			.as_ref()
			.unwrap()
			.statements
			.as_slice()
		else {
			return false;
		};
		todo!("match rest of the function")
	}
}
