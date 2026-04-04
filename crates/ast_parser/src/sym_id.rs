use std::convert::Infallible;

use oxc::{
	ast::ast::{BindingIdentifier, IdentifierReference},
	semantic::{Reference, ReferenceId, Semantic, SymbolId},
};

pub trait GetSymId {
	fn get_sym_id(&self, sema: &Semantic<'_>) -> Option<SymbolId>;
}

impl GetSymId for SymbolId {
	fn get_sym_id(&self, _: &Semantic<'_>) -> Option<SymbolId> {
		Some(*self)
	}
}

impl GetSymId for Reference {
	fn get_sym_id(&self, _: &Semantic<'_>) -> Option<SymbolId> {
		self.symbol_id()
	}
}

impl GetSymId for ReferenceId {
	fn get_sym_id(&self, sema: &Semantic<'_>) -> Option<SymbolId> {
		sema.scoping()
			.get_reference(*self)
			.symbol_id()
	}
}

impl GetSymId for IdentifierReference<'_> {
	fn get_sym_id(&self, sema: &Semantic<'_>) -> Option<SymbolId> {
		sema.scoping()
			.get_reference(self.reference_id())
			.symbol_id()
	}
}

impl GetSymId for BindingIdentifier<'_> {
	fn get_sym_id(&self, _: &Semantic<'_>) -> Option<SymbolId> {
		Some(self.symbol_id())
	}
}

impl GetSymId for Option<Infallible> {
	fn get_sym_id(&self, _: &Semantic<'_>) -> Option<SymbolId> {
		None
	}
}
