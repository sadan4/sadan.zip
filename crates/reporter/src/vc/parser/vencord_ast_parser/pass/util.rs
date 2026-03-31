use crate::vc::parser::exts::TemplateLiteralExt;
use derive_more::{Deref, DerefMut};
use itertools::Itertools;
use oxc::allocator::{Allocator, AllocatorAccessor, Dummy, StringBuilder};
use oxc::ast::ast::{
    BigintBase, Expression, IdentifierReference, NumberBase, TemplateElementValue, TemplateLiteral,
};
use oxc::minifier::PropertyReadSideEffects;
use oxc::semantic::IsGlobalReference;
use oxc::span::{Atom, Span};
use oxc_ecmascript::GlobalContext;
use oxc_ecmascript::constant_evaluation::{
    ConstantEvaluation, ConstantEvaluationCtx, ConstantValue,
};
use oxc_ecmascript::side_effects::MayHaveSideEffectsContext;
use oxc_traverse::TraverseCtx;
use std::borrow::Cow;
use std::mem;

#[derive(Deref, DerefMut)]
#[repr(transparent)]
pub struct Ctx<'a, 'ast: 'a, State>(pub &'a mut TraverseCtx<'ast, State>);

impl<'a, 'ast: 'a, State> Ctx<'a, 'ast, State> {
    pub fn node_from_constant_value(
        &self,
        value: ConstantValue<'ast>,
        span: Span,
    ) -> Expression<'ast> {
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

    /// A quick shortcut to get access to the [`Allocator`]
    pub fn a(&self) -> &'ast Allocator {
        self.ast.allocator
    }

    /// Evalualate a template that only has literal values
    /// # Panics
    /// Panics if the template has non-literal values
    /// Use [`TemplateLiteralExt::is_literal`] to check if a template is a literal
    pub fn eval_template(&self, t: &TemplateLiteral<'ast>) -> Atom<'ast> {
        debug_assert!(
            !t.is_no_substitution_template(),
            "no substution templates should be handled outside of this function to better preserve AST span and source info"
        );
        debug_assert_eq!(
            t.quasis.len(),
            t.expressions.len() + 1,
            "malformed template literal {}",
            t.dbg_str()
        );
        let q_strs = t
            .quasis
            .iter()
            .map(|q| Cow::Borrowed(q.value.cooked.unwrap().as_str()));
        let expr_strs = t
            .expressions
            .iter()
            .map(|e| e.evaluate_value_to_string(self).unwrap());
        let strs = q_strs.interleave(expr_strs);
        let mut ret = StringBuilder::new_in(self.a());
        for s in strs {
            ret.push_str(&s)
        }
        ret.into()
    }
    /// used with [`std::mem::replace`]
    /// ```
    /// fn do_something<'ast>(
    ///     node: &'ast mut Expression<'ast>,
    ///     ctx: Ctx<'_, 'ast, ()>
    /// ) -> Expression<'ast> {
    ///     use std::mem;
    ///     let new_node = mem::replace(node, ctx.dummy());
    ///     new_node
    /// }
    /// ```
    pub fn dummy<T: Dummy<'ast>>(&self) -> T {
        T::dummy(self.ast.allocator)
    }
    pub fn take<T: Dummy<'ast>>(&self, node: &mut T) -> T {
        mem::replace(node, self.dummy())
    }
    pub fn empty_template_element_value(&self) -> TemplateElementValue<'static> {
        TemplateElementValue {
            raw: Atom::from(""),
            cooked: Some(Atom::from("")),
        }
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

impl<'a, 'ast: 'a, State> AllocatorAccessor<'ast> for &Ctx<'a, 'ast, State> {
    fn allocator(self) -> &'ast Allocator {
        self.ast.allocator
    }
}

impl<'a, 'ast: 'a, State> ConstantEvaluationCtx<'ast> for Ctx<'a, 'ast, State> {
    fn ast(&self) -> oxc::ast::AstBuilder<'ast> {
        self.0.ast
    }
}
