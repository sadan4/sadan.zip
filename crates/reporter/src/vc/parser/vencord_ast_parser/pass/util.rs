use derive_more::{Deref, DerefMut};
use oxc::allocator::Dummy;
use oxc::ast::ast::{BigintBase, Expression, IdentifierReference, NumberBase};
use oxc::minifier::PropertyReadSideEffects;
use oxc::semantic::IsGlobalReference;
use oxc::span::Span;
use oxc_ecmascript::GlobalContext;
use oxc_ecmascript::constant_evaluation::{ConstantEvaluationCtx, ConstantValue};
use oxc_ecmascript::side_effects::MayHaveSideEffectsContext;
use oxc_traverse::TraverseCtx;

#[derive(Deref, DerefMut)]
#[repr(transparent)]
pub struct Ctx<'a, 'ast: 'a, State>(pub &'a mut TraverseCtx<'ast, State>);

impl<'a, 'ast: 'a, State> Ctx<'a, 'ast, State> {
    pub fn node_from_constant_value(&self, value: ConstantValue<'ast>, span: Span) -> Expression<'ast> {
        match value {
            ConstantValue::Number(n) => {
                self.ast
                    .expression_numeric_literal(span, n, None, NumberBase::Float)
            }
            ConstantValue::BigInt(n) => {
                let str_repr = self.ast.allocator.alloc_str(&n.to_string());
                self.ast
                    .expression_big_int_literal(span, str_repr, None, BigintBase::Decimal)
            }
            ConstantValue::String(cow) => {
                let atom = self.ast.atom_from_cow(&cow);
                self.ast.expression_string_literal(span, atom, None)
            }
            ConstantValue::Boolean(b) => self.ast.expression_boolean_literal(span, b),
            ConstantValue::Undefined => self.ast.void_0(span),
            ConstantValue::Null => self.ast.expression_null_literal(span),
        }
    }
    
    pub fn dummy<T: Dummy<'ast>>(&self) -> T {
        T::dummy(self.ast.allocator)
    }
}

impl<'a, 'ast: 'a, State> GlobalContext<'ast> for Ctx<'a, 'ast, State> {
    fn is_global_reference(&self, reference: &IdentifierReference<'ast>) -> bool {
        reference.is_global_reference(self.scoping())
    }
}

impl<'a, 'ast: 'a, State> MayHaveSideEffectsContext<'ast> for Ctx<'a, 'ast, State> {
    fn annotations(&self) -> bool {
        true
    }

    fn manual_pure_functions(&self, _callee: &oxc::ast::ast::Expression) -> bool {
        false
    }

    fn property_read_side_effects(&self) -> PropertyReadSideEffects {
        PropertyReadSideEffects::None
    }

    fn unknown_global_side_effects(&self) -> bool {
        false
    }
}

impl<'a, 'ast: 'a, State> ConstantEvaluationCtx<'ast> for Ctx<'a, 'ast, State> {
    fn ast(&self) -> oxc::ast::AstBuilder<'ast> {
        self.0.ast
    }
}
