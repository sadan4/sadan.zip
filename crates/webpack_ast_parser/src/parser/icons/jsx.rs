use oxc::{
	ast::ast::{Expression, ObjectExpression, StringLiteral},
	span::{GetSpan, Span},
	syntax::GetNodeId,
};

#[derive(Clone, Copy, Debug)]
pub struct Call<'ast> {
	pub tag: Tag<'ast>,
	pub props: &'ast ObjectExpression<'ast>,
	pub key: Option<&'ast Expression<'ast>>,
}

impl GetSpan for Call<'_> {
	fn span(&self) -> Span {
		let s = self.tag.span().merge(self.props.span());
		if let Some(k) = self.key {
			s.merge(k.span())
		} else {
			s
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub enum Tag<'ast> {
	Dom(&'ast StringLiteral<'ast>),
	Component(&'ast Expression<'ast>),
}

impl GetNodeId for Tag<'_> {
	fn node_id(&self) -> oxc::semantic::NodeId {
		match self {
			Tag::Dom(s) => s.node_id(),
			Tag::Component(s) => s.node_id(),
		}
	}
}

impl GetSpan for Tag<'_> {
	fn span(&self) -> Span {
		match self {
			Tag::Dom(s) => s.span(),
			Tag::Component(s) => s.span(),
		}
	}
}
