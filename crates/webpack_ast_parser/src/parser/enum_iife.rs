use ast_parser::{
	AstParser as _,
	ast_kind::IntoAstKind as _,
	exts::ExpressionExt as _,
};
use oxc::{
	ast::ast::{
		AssignmentExpression,
		AssignmentTarget,
		ComputedMemberExpression,
		Expression,
		MemberExpression,
		StaticMemberExpression,
	},
	semantic::SymbolId,
};
use smol_str::SmolStr;

use crate::{WebpackAstParser, parser::export_map::RawExportMap};

pub struct EnumIIFEState1_2<'a, 'ast> {
	pub p: &'a WebpackAstParser<'ast>,
	pub enum_param: SymbolId,
	pub ret: RawExportMap<'ast>,
}

impl<'ast> EnumIIFEState1_2<'_, 'ast> {
	/// Resolve the enum object an entry is assigned onto.
	///
	/// This is either a bare identifier `e` (IIFE style, and every entry
	/// after the first in the sequence-expression style), or the
	/// first-entry initializer `(e = {})` used by the sequence-expression
	/// style, where the enum object is created inline.
	fn resolve_enum_object(
		&self,
		obj: &'ast Expression<'ast>,
	) -> Option<SymbolId> {
		match obj.get_inner_expression() {
			Expression::Identifier(id) => self.p.sym_id_of(id.as_ref()),
			// `(e = {})` in `(e = {}).KEY = val`
			Expression::AssignmentExpression(assign) => {
				if !assign
					.right
					.as_object_expression()?
					.properties
					.is_empty()
				{
					return None;
				}
				match &assign.left {
					AssignmentTarget::AssignmentTargetIdentifier(id) => {
						self.p.sym_id_of(&id.reference_id())
					}
					_ => None,
				}
			}
			_ => None,
		}
	}
	fn process_style_1(
		&mut self,
		left: &'ast StaticMemberExpression<'ast>,
		right: &'ast Expression<'ast>,
	) -> Option<()> {
		let param_sym = self.resolve_enum_object(&left.object)?;
		let entry_name = &left.property;
		if param_sym != self.enum_param {
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
		let param_sym = self.resolve_enum_object(&left.object)?;
		let style_1_entry = left
			.expression
			.as_assignment_expression()?;
		// TODO: debug assert names match between this and the rhs
		if param_sym != self.enum_param {
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
