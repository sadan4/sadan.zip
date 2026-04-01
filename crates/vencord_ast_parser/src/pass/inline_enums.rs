use super::util::Ctx;
use ast_parser::exts::ExpressionExt as _;
use derive_more::{Deref, DerefMut};
use oxc::{
	allocator::{Allocator, CloneIn},
	ast::ast::{Expression, IdentifierReference, NumberBase, TSEnumMemberName},
	minifier::PropertyReadSideEffects,
	semantic::{ReferenceId, SymbolId},
	span::{Atom, GetSpan, Span},
};
use oxc_ecmascript::{
	GlobalContext,
	constant_evaluation::{
		ConstantEvaluation, ConstantEvaluationCtx, ConstantValue,
	},
	side_effects::MayHaveSideEffectsContext,
};
use oxc_traverse::Traverse;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct InlineEnumsPass<'ast> {
	value_map: HashMap<SymbolId, HashMap<Atom<'ast>, (Span, EnumValue<'ast>)>>,
}

/// Recursively inline enum member references within an expression
/// This is used when processing enum initializers that reference other enum members
fn inline_enum_references_in_expr<'ast>(
	expr: &Expression<'ast>,
	tracker: &EnumValueTracker<'_, 'ast>,
	enum_symbol_id: SymbolId,
	value_map: &HashMap<Atom<'ast>, (Span, EnumValue<'ast>)>,
	ctx: &Ctx<'_, 'ast, ()>,
) -> Expression<'ast> {
	match expr {
		// Handle static member access like `Colors.Blue`
		Expression::StaticMemberExpression(member_expr) => {
			// Check if this is a reference to the current enum
			if let Some(enum_obj) = member_expr.object.as_identifier()
				&& let Some(ref_sym_id) = ctx
					.scoping()
					.get_reference(enum_obj.reference_id())
					.symbol_id()
				&& ref_sym_id == enum_symbol_id
			{
				// This is a reference to our enum, try to inline it
				if let Some((span, enum_value)) =
					value_map.get(&member_expr.property.name.as_atom())
				{
					return enum_value_to_expression(enum_value, *span, ctx);
				}
			}
			// If we can't inline it, clone it as-is
			expr.clone_in(ctx.a())
		}
		// Handle identifier references like `Red`
		Expression::Identifier(ident) => {
			// Check if this identifier is in the tracker (i.e., it's a reference to an enum member)
			let ref_id = ident.reference_id();
			if let Some(enum_value) = tracker.0.get(&ref_id) {
				return enum_value_to_expression(enum_value, ident.span(), ctx);
			}
			// If not in tracker, clone as-is
			expr.clone_in(ctx.a())
		}
		// For binary expressions, recursively process both sides
		Expression::BinaryExpression(bin_expr) => {
			let left = inline_enum_references_in_expr(
				&bin_expr.left,
				tracker,
				enum_symbol_id,
				value_map,
				ctx,
			);
			let right = inline_enum_references_in_expr(
				&bin_expr.right,
				tracker,
				enum_symbol_id,
				value_map,
				ctx,
			);
			ctx.ast.expression_binary(
				bin_expr.span,
				left,
				bin_expr.operator,
				right,
			)
		}
		// For other expression types, clone as-is
		// TODO: Add more expression types if needed (unary, parenthesized, etc.)
		_ => expr.clone_in(ctx.a()),
	}
}

/// Convert an [`EnumValue`] to an Expression
fn enum_value_to_expression<'ast>(
	value: &EnumValue<'ast>,
	span: Span,
	ctx: &Ctx<'_, 'ast, ()>,
) -> Expression<'ast> {
	match value {
		EnumValue::Number(n) => ctx.ast.expression_numeric_literal(
			span,
			*n,
			None,
			NumberBase::Decimal,
		),
		EnumValue::String(atom) => ctx
			.ast
			.expression_string_literal(span, *atom, None),
		EnumValue::Computed(expr) => expr.clone_in(ctx.a()),
	}
}

#[derive(Debug)]
enum EnumValue<'ast> {
	Number(f64),
	String(Atom<'ast>),
	Computed(Expression<'ast>),
}

impl<'new> CloneIn<'new> for EnumValue<'_> {
	type Cloned = EnumValue<'new>;

	fn clone_in(&self, alloc: &'new Allocator) -> Self::Cloned {
		match self {
			EnumValue::Number(n) => EnumValue::Number(*n),
			EnumValue::String(s) => EnumValue::String(s.clone_in(alloc)),
			EnumValue::Computed(e) => EnumValue::Computed(e.clone_in(alloc)),
		}
	}

	fn clone_in_with_semantic_ids(
		&self,
		allocator: &'new Allocator,
	) -> Self::Cloned {
		match self {
			EnumValue::Number(n) => EnumValue::Number(*n),
			EnumValue::String(s) => {
				EnumValue::String(s.clone_in_with_semantic_ids(allocator))
			}
			EnumValue::Computed(e) => {
				EnumValue::Computed(e.clone_in_with_semantic_ids(allocator))
			}
		}
	}
}

#[derive(Deref, DerefMut)]
struct EnumValueTracker<'a, 'ast>(
	#[deref]
	#[deref_mut]
	// TODO: make this a HashMap<SymbolId, EnumValue<'ast>> for less memory usage
	HashMap<ReferenceId, EnumValue<'ast>>,
	&'a Ctx<'a, 'ast, ()>,
);

impl<'ast> Traverse<'ast, ()> for InlineEnumsPass<'ast> {
	/// resolve each enum value to an in-linable expression
	fn exit_ts_enum_declaration(
		&mut self,
		node: &mut oxc::ast::ast::TSEnumDeclaration<'ast>,
		ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
	) {
		if node.declare {
			return;
		}
		let ctx = Ctx(ctx);
		// TypeScript enums start at 0, so there is an implicit last value of -1
		let mut last_value = Some(-1);
		let mut tracker = EnumValueTracker::new(&ctx);
		let mut value_map = HashMap::new();
		let enum_scope_id = node.body.scope_id();
		let enum_symbol_id = node.id.symbol_id();

		for v in &node.body.members {
			let (span, value) = match (&mut last_value, &v.initializer) {
				// something like
				// ```ts
				// const NUM = 50;
				// enum Foo {
				//     A = NUM,
				//     B,
				// }
				// ```
				(None, None) => return,
				(_, Some(expr)) => {
					// First evaluate the expression to see if we can get a constant value
					let evaluated = expr.evaluate_value(&tracker);

					let value = match evaluated {
						Some(ConstantValue::String(s)) => {
							let atom = Atom::from_cow_in(&s, ctx.a());
							EnumValue::String(atom)
						}
						Some(ConstantValue::Number(n)) => {
							debug_assert!(n.is_finite() && n.fract() == 0.);
							last_value = Some(n as i32);
							EnumValue::Number(n)
						}
						None => {
							// If we can't evaluate it to a constant, we need to inline any enum references
							let inlined_expr = inline_enum_references_in_expr(
								expr,
								&tracker,
								enum_symbol_id,
								&value_map,
								&ctx,
							);
							last_value = None;
							EnumValue::Computed(inlined_expr)
						}
						Some(_) => panic!(
							"Invalid Enum. Constant enum member initializers must evaluate to a string or a numeric literal."
						),
					};

					(expr.span(), value)
				}
				(Some(prev), None) => {
					*prev += 1;
					(v.id.span(), EnumValue::Number(f64::from(*prev)))
				}
			};

			// the other types of member names don't matter because they can't be referenced
			if let TSEnumMemberName::Identifier(id) = &v.id {
				// oxc does not bind enum member identifiers to symbol ids, so we have to find them
				let sym_id = ctx
					.scoping()
					.get_binding(enum_scope_id, id.name)
					.expect("Failed to lookup enum name sym_id");
				for ref_id in ctx
					.scoping()
					.get_resolved_reference_ids(sym_id)
				{
					tracker.insert(*ref_id, value.clone_in(ctx.a()));
				}
			}
			value_map.insert(v.id.static_name(), (span, value));
		}
		self.value_map
			.insert(node.id.symbol_id(), value_map);
	}
	fn enter_expression(
		&mut self,
		node: &mut Expression<'ast>,
		ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
	) {
		let ctx = Ctx(ctx);
		let Expression::StaticMemberExpression(expr) = node else {
			return;
		};
		let Some(enum_obj) = expr.object.as_identifier_mut() else {
			return;
		};
		let Some(enum_sym_id) = ctx
			.scoping()
			.get_reference(enum_obj.reference_id())
			.symbol_id()
		else {
			return;
		};
		let Some(enum_value_map) = self.value_map.get(&enum_sym_id) else {
			return;
		};
		let Some((span, constant_value)) =
			enum_value_map.get(&expr.property.name.as_atom())
		else {
			return;
		};
		match constant_value {
			EnumValue::Number(n) => {
				*node = ctx.ast.expression_numeric_literal(
					*span,
					*n,
					None,
					NumberBase::Decimal,
				);
			}
			EnumValue::String(atom) => {
				*node = ctx
					.ast
					.expression_string_literal(*span, *atom, None);
			}
			EnumValue::Computed(expr) => {
				*node = expr.clone_in(ctx.a());
			}
		}
	}
}

impl<'a, 'ast> EnumValueTracker<'a, 'ast> {
	fn new(ctx: &'a Ctx<'a, 'ast, ()>) -> Self {
		Self(HashMap::new(), ctx)
	}
}

impl<'ast> GlobalContext<'ast> for EnumValueTracker<'_, 'ast> {
	fn is_global_reference(
		&self,
		reference: &IdentifierReference<'ast>,
	) -> bool {
		self.contains_key(&reference.reference_id())
			|| self.1.is_global_reference(reference)
	}
	fn get_constant_value_for_reference_id(
		&self,
		ref_id: ReferenceId,
	) -> Option<ConstantValue<'ast>> {
		match self.0.get(&ref_id) {
			Some(EnumValue::Number(e)) => Some(ConstantValue::Number(*e)),
			Some(EnumValue::String(s)) => {
				Some(ConstantValue::String((*s).into()))
			}
			Some(EnumValue::Computed(_)) => None,
			None => self
				.1
				.get_constant_value_for_reference_id(ref_id),
		}
	}
}

impl<'ast> MayHaveSideEffectsContext<'ast> for EnumValueTracker<'_, 'ast> {
	fn annotations(&self) -> bool {
		self.1.annotations()
	}

	fn manual_pure_functions(&self, callee: &Expression) -> bool {
		self.1.manual_pure_functions(callee)
	}

	fn property_read_side_effects(&self) -> PropertyReadSideEffects {
		self.1.property_read_side_effects()
	}

	fn unknown_global_side_effects(&self) -> bool {
		self.1.unknown_global_side_effects()
	}
}

impl<'ast> ConstantEvaluationCtx<'ast> for EnumValueTracker<'_, 'ast> {
	fn ast(&self) -> oxc::ast::AstBuilder<'ast> {
		self.1.ast()
	}
}

#[cfg(test)]
mod tests {
	#![allow(clippy::needless_raw_string_hashes)]
	use insta::assert_snapshot;

	use crate::test_pass;

	use super::*;

	#[test]
	fn test_basic_numeric_enum() {
		let code = /* language=Typescript */ r#"
            enum Direction {
                Up,
                Down,
                Left,
                Right
            }
            const x = Direction.Up;
            const y = Direction.Down;
            const z = Direction.Right;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Direction {
			Up,
			Down,
			Left,
			Right
		}
		const x = 0;
		const y = 1;
		const z = 3;
		");
	}

	#[test]
	fn test_numeric_enum_with_initializers() {
		let code = /* language=Typescript */ r#"
            enum Status {
                Ready = 10,
                Waiting,
                Done = 100,
                Error
            }
            const a = Status.Ready;
            const b = Status.Waiting;
            const c = Status.Done;
            const d = Status.Error;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Status {
			Ready = 10,
			Waiting,
			Done = 100,
			Error
		}
		const a = 10;
		const b = 11;
		const c = 100;
		const d = 101;
		");
	}

	#[test]
	fn test_string_enum() {
		let code = /* language=Typescript */ r#"
            enum LogLevel {
                ERROR = "error",
                WARN = "warn",
                INFO = "info",
                DEBUG = "debug"
            }
            const level = LogLevel.ERROR;
            const info = LogLevel.INFO;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @"
        enum LogLevel {
        	ERROR = 'error',
        	WARN = 'warn',
        	INFO = 'info',
        	DEBUG = 'debug'
        }
        const level = 'error';
        const info = 'info';
        ");
	}

	#[test]
	fn test_mixed_enum() {
		let code = /* language=Typescript */ r#"
            enum Mixed {
                A,
                B = "string",
                C = 10,
                D
            }
            const a = Mixed.A;
            const b = Mixed.B;
            const c = Mixed.C;
            const d = Mixed.D;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @"
        enum Mixed {
        	A,
        	B = 'string',
        	C = 10,
        	D
        }
        const a = 0;
        const b = 'string';
        const c = 10;
        const d = 11;
        ");
	}

	#[test]
	fn test_computed_enum() {
		let code = /* language=Typescript */ r#"
            const BASE = 100;
            enum Computed {
                A = BASE,
                B = BASE + 1,
            }
            const x = Computed.A;
            const y = Computed.B;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		const BASE = 100;
		enum Computed {
			A = BASE,
			B = BASE + 1
		}
		const x = BASE;
		const y = BASE + 1;
		");
	}

	#[test]
	fn test_enum_with_expression_initializer() {
		let code = /* language=Typescript */ r#"
            enum Flags {
                None = 0,
                Read = 1 << 0,
                Write = 1 << 1,
                Execute = 1 << 2
            }
            const perms = Flags.Read;
            const write = Flags.Write;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Flags {
			None = 0,
			Read = 1 << 0,
			Write = 1 << 1,
			Execute = 1 << 2
		}
		const perms = 1;
		const write = 2;
		");
	}

	#[test]
	fn test_enum_member_references_within_enum() {
		let code = /* language=Typescript */ r#"
            enum Colors {
                Red = 0,
                Green = 1,
                Blue = 2,
                Purple = Red + Colors.Blue
            }
            const color = Colors.Purple;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Colors {
			Red = 0,
			Green = 1,
			Blue = 2,
			Purple = Red + Colors.Blue
		}
		const color = 0 + 2;
		");
	}

	#[test]
	fn test_negative_enum_values() {
		let code = /* language=Typescript */ r#"
            enum Temperature {
                Freezing = -32,
                Cold,
                Cool = 0,
                Warm,
                Hot = 100
            }
            const temp1 = Temperature.Freezing;
            const temp2 = Temperature.Cold;
            const temp3 = Temperature.Warm;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Temperature {
			Freezing = -32,
			Cold,
			Cool = 0,
			Warm,
			Hot = 100
		}
		const temp1 = -32;
		const temp2 = -31;
		const temp3 = 1;
		");
	}

	#[test]
	fn test_multiple_enum_references() {
		let code = /* language=Typescript */ r#"
            enum Status {
                Idle,
                Active,
                Done
            }
            const a = Status.Idle;
            const b = Status.Active;
            const c = Status.Idle;
            const d = Status.Done;
            const e = Status.Idle;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Status {
			Idle,
			Active,
			Done
		}
		const a = 0;
		const b = 1;
		const c = 0;
		const d = 2;
		const e = 0;
		");
	}

	#[test]
	fn test_enum_does_not_inline_non_static_member() {
		let code = /* language=Typescript */ r#"
            enum Test {
                A,
                B
            }
            const x = Test['A'];
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Test {
			A,
			B
		}
		const x = Test['A'];
		");
	}

	#[test]
	fn test_enum_in_expression() {
		let code = /* language=Typescript */ r#"
            enum Numbers {
                One = 1,
                Two = 2,
                Three = 3
            }
            const sum = Numbers.One + Numbers.Two + Numbers.Three;
            const product = Numbers.Two * Numbers.Three;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Numbers {
			One = 1,
			Two = 2,
			Three = 3
		}
		const sum = 1 + 2 + 3;
		const product = 2 * 3;
		");
	}

	#[test]
	fn test_enum_with_template_literal_initializer() {
		let code = /* language=Typescript */ r#"
            const prefix = "value_";
            enum Keys {
                A = `${prefix}a`,
                B = `${prefix}b`
            }
            const key = Keys.A;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		const prefix = 'value_';
		enum Keys {
			A = `${prefix}a`,
			B = `${prefix}b`
		}
		const key = `${prefix}a`;
		");
	}

	#[test]
	fn test_declare_enum_is_not_inlined() {
		let code = /* language=Typescript */ r#"
            declare enum External {
                A,
                B
            }
            const x = External.A;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		declare enum External {
			A,
			B
		}
		const x = External.A;
		");
	}

	#[test]
	fn test_multiple_enums() {
		let code = /* language=Typescript */ r#"
            enum First {
                A = 1,
                B = 2
            }
            enum Second {
                X = "x",
                Y = "y"
            }
            const a = First.A;
            const x = Second.X;
            const b = First.B;
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @"
        enum First {
        	A = 1,
        	B = 2
        }
        enum Second {
        	X = 'x',
        	Y = 'y'
        }
        const a = 1;
        const x = 'x';
        const b = 2;
        ");
	}

	#[test]
	fn test_enum_in_function_call() {
		let code = /* language=Typescript */ r#"
            enum Priority {
                Low = 0,
                Medium = 1,
                High = 2
            }
            function setPriority(p: number) {}
            setPriority(Priority.High);
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		enum Priority {
			Low = 0,
			Medium = 1,
			High = 2
		}
		function setPriority(p: number) {}
		setPriority(2);
		");
	}

	#[test]
	fn test_enum_in_array() {
		let code = /* language=Typescript */ r#"
            enum Color {
                Red,
                Green,
                Blue
            }
            const colors = [Color.Red, Color.Green, Color.Blue];
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @"
        enum Color {
        	Red,
        	Green,
        	Blue
        }
        const colors = [
        	0,
        	1,
        	2
        ];
        ");
	}

	#[test]
	fn test_enum_in_object() {
		let code = /* language=Typescript */ r#"
            enum Status {
                Pending = "pending",
                Complete = "complete"
            }
            const obj = {
                status: Status.Pending,
                final: Status.Complete
            };
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @"
        enum Status {
        	Pending = 'pending',
        	Complete = 'complete'
        }
        const obj = {
        	status: 'pending',
        	final: 'complete'
        };
        ");
	}

	#[test]
	fn test_scoped_enums() {
		let code = /* language=Typescript */ r#"
            {
                enum Inner {
                    A = 1
                }
                const x = Inner.A;
            }
            {
                enum Inner {
                    A = 2
                }
                const y = Inner.A;
            }
        "#;
		let out = test_pass!(code, InlineEnumsPass::default());
		assert_snapshot!(out, @r"
		{
			enum Inner {
				A = 1
			}
			const x = 1;
		}
		{
			enum Inner {
				A = 2
			}
			const y = 2;
		}
		");
	}
}
