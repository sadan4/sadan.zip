use derive_more::{Deref, DerefMut};
use oxc::ast::ast::IdentifierReference;
use oxc::semantic::IsGlobalReference;
use oxc_ecmascript::GlobalContext;
use oxc_traverse::TraverseCtx;

#[derive(Deref, DerefMut)]
#[repr(transparent)]
pub struct Ctx<'a, 'ast: 'a, State>(pub &'a mut TraverseCtx<'ast, State>);

impl<'a, 'ast: 'a, State> GlobalContext<'ast> for Ctx<'a, 'ast, State> {
    fn is_global_reference(&self, reference: &IdentifierReference<'ast>) -> bool {
        reference.is_global_reference(self.scoping())
    }
}
