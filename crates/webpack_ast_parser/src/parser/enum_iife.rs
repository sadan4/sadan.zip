use ast_parser::{
	AstParser as _,
	ast_kind::IntoAstKind as _,
	exts::ExpressionExt as _,
};
use oxc::ast::ast::{
	AssignmentExpression,
	BindingIdentifier,
	ComputedMemberExpression,
	Expression,
	MemberExpression,
	StaticMemberExpression,
};
use smol_str::SmolStr;

use crate::{WebpackAstParser, parser::export_map::RawExportMap};

pub struct EnumIIFEState1_2<'a, 'ast> {
	pub p: &'a WebpackAstParser<'ast>,
	pub enum_param: &'ast BindingIdentifier<'ast>,
	pub ret: RawExportMap<'ast>,
}

impl<'ast> EnumIIFEState1_2<'_, 'ast> {
	fn process_style_1(
		&mut self,
		left: &'ast StaticMemberExpression<'ast>,
		right: &'ast Expression<'ast>,
	) -> Option<()> {
		let param_use = left.object.as_identifier()?;
		let entry_name = &left.property;
		if !self
			.p
			.cmp_sym(param_use, self.enum_param)
		{
			return None;
		}
		let mut export_range = self
			.p
			.raw_make_export_map_recursive(entry_name)
			.unwrap_range();
		export_range.push(right.into_ast_kind());

		let key = SmolStr::new(entry_name.name.as_str());
		debug_assert!(
			!self.ret.exports.contains_key(&key),
			"Duplicate export name while parsing enum iife"
		);
		// doesn't cover no-sub templates, but not really important here
		debug_assert!(
			right
				.as_template_literal()
				.is_none_or(|t| !t.is_no_substitution_template())
		);
		if right.is_literal() {
			export_range.annotate(SmolStr::new(self.p.text(right)));
		}

		self.ret
			.exports
			.insert(key, export_range.into());
		Some(())
	}
	fn process_style_2(
		&mut self,
		left: &'ast ComputedMemberExpression<'ast>,
	) -> Option<()> {
		let param_use = left.object.as_identifier()?;
		let style_1_entry = left
			.expression
			.as_assignment_expression()?;
		// TODO: debug assert names match between this and the rhs
		if !self
			.p
			.cmp_sym(param_use, self.enum_param)
		{
			return None;
		}

		self.process(style_1_entry)
	}
	pub fn process(
		&mut self,
		node: &'ast AssignmentExpression<'ast>,
	) -> Option<()> {
		let left = node.left.as_member_expression()?;
		let right = &node.right;
		match left {
			MemberExpression::StaticMemberExpression(left) => {
				self.process_style_1(left, right)
			}
			MemberExpression::ComputedMemberExpression(left) => {
				self.process_style_2(left)
			}
			MemberExpression::PrivateFieldExpression(_) => None,
		}
	}
}
