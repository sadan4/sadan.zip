use oxc::{
	ast::{
		AstKind,
		ast::{
			BindingPattern,
			Expression as E,
			MemberExpression,
			PropertyKey,
			Statement as S,
		},
	},
	semantic::AstNode,
};

/// Generic trait for anything that be represented as an [`AstKind`].
pub trait IntoAstKind<'ast> {
	/// Convert `self` into an [`AstKind`].
	fn into_ast_kind(self) -> AstKind<'ast>;
}

impl<'a> IntoAstKind<'a> for AstKind<'a> {
	fn into_ast_kind(self) -> Self {
		self
	}
}

impl<'a> IntoAstKind<'a> for AstNode<'a> {
	fn into_ast_kind(self) -> AstKind<'a> {
		self.kind()
	}
}

macro_rules! make_impl {
	($kind:ident) => {
		impl<'a> IntoAstKind<'a> for &'a oxc::ast::ast::$kind<'a> {
			fn into_ast_kind(self) -> AstKind<'a> {
				AstKind::$kind(self)
			}
		}
	};
}

macro_rules! make_impl_no_lt {
	($kind:ident) => {
		impl<'a> IntoAstKind<'a> for &'a oxc::ast::ast::$kind {
			fn into_ast_kind(self) -> AstKind<'a> {
				AstKind::$kind(self)
			}
		}
	};
}

make_impl!(Program);
make_impl!(IdentifierName);
make_impl!(IdentifierReference);
make_impl!(BindingIdentifier);
make_impl!(LabelIdentifier);
make_impl_no_lt!(ThisExpression);
make_impl!(ArrayExpression);
make_impl_no_lt!(Elision);
make_impl!(ObjectExpression);
make_impl!(ObjectProperty);
make_impl!(TemplateLiteral);
make_impl!(TaggedTemplateExpression);
make_impl!(TemplateElement);
make_impl!(ComputedMemberExpression);
make_impl!(StaticMemberExpression);
make_impl!(PrivateFieldExpression);
make_impl!(CallExpression);
make_impl!(NewExpression);
make_impl_no_lt!(ImportMeta);
make_impl_no_lt!(NewTarget);
make_impl!(SpreadElement);
make_impl!(UpdateExpression);
make_impl!(UnaryExpression);
make_impl!(BinaryExpression);
make_impl!(PrivateInExpression);
make_impl!(LogicalExpression);
make_impl!(ConditionalExpression);
make_impl!(AssignmentExpression);
make_impl!(ArrayAssignmentTarget);
make_impl!(ObjectAssignmentTarget);
make_impl!(AssignmentTargetRest);
make_impl!(AssignmentTargetWithDefault);
make_impl!(AssignmentTargetPropertyIdentifier);
make_impl!(AssignmentTargetPropertyProperty);
make_impl!(SequenceExpression);
make_impl_no_lt!(Super);
make_impl!(AwaitExpression);
make_impl!(ChainExpression);
make_impl!(ParenthesizedExpression);
make_impl!(Directive);
make_impl!(Hashbang);
make_impl!(BlockStatement);
make_impl!(VariableDeclaration);
make_impl!(VariableDeclarator);
make_impl_no_lt!(EmptyStatement);
make_impl!(ExpressionStatement);
make_impl!(IfStatement);
make_impl!(DoWhileStatement);
make_impl!(WhileStatement);
make_impl!(ForStatement);
make_impl!(ForInStatement);
make_impl!(ForOfStatement);
make_impl!(ContinueStatement);
make_impl!(BreakStatement);
make_impl!(ReturnStatement);
make_impl!(WithStatement);
make_impl!(SwitchStatement);
make_impl!(SwitchCase);
make_impl!(LabeledStatement);
make_impl!(ThrowStatement);
make_impl!(TryStatement);
make_impl!(CatchClause);
make_impl!(CatchParameter);
make_impl_no_lt!(DebuggerStatement);
make_impl!(AssignmentPattern);
make_impl!(ObjectPattern);
make_impl!(BindingProperty);
make_impl!(ArrayPattern);
make_impl!(BindingRestElement);
make_impl!(Function);
make_impl!(FormalParameters);
make_impl!(FormalParameter);
make_impl!(FormalParameterRest);
make_impl!(FunctionBody);
make_impl!(ArrowFunctionExpression);
make_impl!(YieldExpression);
make_impl!(Class);
make_impl!(ClassBody);
make_impl!(MethodDefinition);
make_impl!(PropertyDefinition);
make_impl!(PrivateIdentifier);
make_impl!(StaticBlock);
make_impl!(AccessorProperty);
make_impl!(ImportExpression);
make_impl!(ImportDeclaration);
make_impl!(ImportSpecifier);
make_impl!(ImportDefaultSpecifier);
make_impl!(ImportNamespaceSpecifier);
make_impl!(WithClause);
make_impl!(ImportAttribute);
make_impl!(ExportDeclaration);
make_impl!(ExportFromDeclaration);
make_impl!(ExportNamedDeclaration);
make_impl!(ExportDefaultDeclaration);
make_impl!(ExportAllDeclaration);
make_impl!(ExportSpecifier);
make_impl!(V8IntrinsicExpression);
make_impl_no_lt!(BooleanLiteral);
make_impl_no_lt!(NullLiteral);
make_impl!(NumericLiteral);
make_impl!(StringLiteral);
make_impl!(BigIntLiteral);
make_impl!(RegExpLiteral);
make_impl!(JSXElement);
make_impl!(JSXOpeningElement);
make_impl!(JSXClosingElement);
make_impl!(JSXFragment);
make_impl_no_lt!(JSXOpeningFragment);
make_impl_no_lt!(JSXClosingFragment);
make_impl!(JSXNamespacedName);
make_impl!(JSXMemberExpression);
make_impl!(JSXExpressionContainer);
make_impl_no_lt!(JSXEmptyExpression);
make_impl!(JSXAttribute);
make_impl!(JSXSpreadAttribute);
make_impl!(JSXIdentifier);
make_impl!(JSXSpreadChild);
make_impl!(JSXText);
make_impl!(TSThisParameter);
make_impl!(TSEnumDeclaration);
make_impl!(TSEnumBody);
make_impl!(TSEnumMember);
make_impl!(TSTypeAnnotation);
make_impl!(TSLiteralType);
make_impl!(TSConditionalType);
make_impl!(TSUnionType);
make_impl!(TSIntersectionType);
make_impl!(TSParenthesizedType);
make_impl!(TSTypeOperator);
make_impl!(TSArrayType);
make_impl!(TSIndexedAccessType);
make_impl!(TSTupleType);
make_impl!(TSNamedTupleMember);
make_impl!(TSOptionalType);
make_impl!(TSRestType);
make_impl_no_lt!(TSAnyKeyword);
make_impl_no_lt!(TSStringKeyword);
make_impl_no_lt!(TSBooleanKeyword);
make_impl_no_lt!(TSNumberKeyword);
make_impl_no_lt!(TSNeverKeyword);
make_impl_no_lt!(TSIntrinsicKeyword);
make_impl_no_lt!(TSUnknownKeyword);
make_impl_no_lt!(TSNullKeyword);
make_impl_no_lt!(TSUndefinedKeyword);
make_impl_no_lt!(TSVoidKeyword);
make_impl_no_lt!(TSSymbolKeyword);
make_impl_no_lt!(TSThisType);
make_impl_no_lt!(TSObjectKeyword);
make_impl_no_lt!(TSBigIntKeyword);
make_impl!(TSTypeReference);
make_impl!(TSQualifiedName);
make_impl!(TSTypeParameterInstantiation);
make_impl!(TSTypeParameter);
make_impl!(TSTypeParameterDeclaration);
make_impl!(TSTypeAliasDeclaration);
make_impl!(TSClassImplements);
make_impl!(TSInterfaceDeclaration);
make_impl!(TSInterfaceBody);
make_impl!(TSPropertySignature);
make_impl!(TSIndexSignature);
make_impl!(TSCallSignatureDeclaration);
make_impl!(TSMethodSignature);
make_impl!(TSConstructSignatureDeclaration);
make_impl!(TSIndexSignatureName);
make_impl!(TSInterfaceHeritage);
make_impl!(TSTypePredicate);
make_impl!(TSNamespaceDeclaration);
make_impl!(TSExternalModuleDeclaration);
make_impl!(TSGlobalDeclaration);
make_impl!(TSModuleBlock);
make_impl!(TSTypeLiteral);
make_impl!(TSInferType);
make_impl!(TSTypeQuery);
make_impl!(TSImportType);
make_impl!(TSImportTypeQualifiedName);
make_impl!(TSFunctionType);
make_impl!(TSConstructorType);
make_impl!(TSMappedType);
make_impl!(TSTemplateLiteralType);
make_impl!(TSAsExpression);
make_impl!(TSSatisfiesExpression);
make_impl!(TSTypeAssertion);
make_impl!(TSImportEqualsDeclaration);
make_impl!(TSExternalModuleReference);
make_impl!(TSNonNullExpression);
make_impl!(Decorator);
make_impl!(TSExportAssignment);
make_impl!(TSNamespaceExportDeclaration);
make_impl!(TSInstantiationExpression);
make_impl!(JSDocNullableType);
make_impl!(JSDocNonNullableType);
make_impl_no_lt!(JSDocUnknownType);

impl<'ast> IntoAstKind<'ast> for &'ast MemberExpression<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			MemberExpression::ComputedMemberExpression(a) => a.into_ast_kind(),
			MemberExpression::StaticMemberExpression(a) => a.into_ast_kind(),
			MemberExpression::PrivateFieldExpression(a) => a.into_ast_kind(),
		}
	}
}

impl<'ast> IntoAstKind<'ast> for &'ast E<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			E::BooleanLiteral(e) => e.into_ast_kind(),
			E::NullLiteral(e) => e.into_ast_kind(),
			E::NumericLiteral(e) => e.into_ast_kind(),
			E::BigIntLiteral(e) => e.into_ast_kind(),
			E::RegExpLiteral(e) => e.into_ast_kind(),
			E::StringLiteral(e) => e.into_ast_kind(),
			E::TemplateLiteral(e) => e.into_ast_kind(),
			E::Identifier(e) => e.into_ast_kind(),
			E::ImportMeta(e) => e.into_ast_kind(),
			E::NewTarget(e) => e.into_ast_kind(),
			E::Super(e) => e.into_ast_kind(),
			E::ArrayExpression(e) => e.into_ast_kind(),
			E::ArrowFunctionExpression(e) => e.into_ast_kind(),
			E::AssignmentExpression(e) => e.into_ast_kind(),
			E::AwaitExpression(e) => e.into_ast_kind(),
			E::BinaryExpression(e) => e.into_ast_kind(),
			E::CallExpression(e) => e.into_ast_kind(),
			E::ChainExpression(e) => e.into_ast_kind(),
			E::ClassExpression(e) => e.into_ast_kind(),
			E::ConditionalExpression(e) => e.into_ast_kind(),
			E::FunctionExpression(e) => e.into_ast_kind(),
			E::ImportExpression(e) => e.into_ast_kind(),
			E::LogicalExpression(e) => e.into_ast_kind(),
			E::NewExpression(e) => e.into_ast_kind(),
			E::ObjectExpression(e) => e.into_ast_kind(),
			E::ParenthesizedExpression(e) => e.into_ast_kind(),
			E::SequenceExpression(e) => e.into_ast_kind(),
			E::TaggedTemplateExpression(e) => e.into_ast_kind(),
			E::ThisExpression(e) => e.into_ast_kind(),
			E::UnaryExpression(e) => e.into_ast_kind(),
			E::UpdateExpression(e) => e.into_ast_kind(),
			E::YieldExpression(e) => e.into_ast_kind(),
			E::PrivateInExpression(e) => e.into_ast_kind(),
			E::JSXElement(e) => e.into_ast_kind(),
			E::JSXFragment(e) => e.into_ast_kind(),
			E::TSAsExpression(e) => e.into_ast_kind(),
			E::TSSatisfiesExpression(e) => e.into_ast_kind(),
			E::TSTypeAssertion(e) => e.into_ast_kind(),
			E::TSNonNullExpression(e) => e.into_ast_kind(),
			E::TSInstantiationExpression(e) => e.into_ast_kind(),
			E::V8IntrinsicExpression(e) => e.into_ast_kind(),
			E::ComputedMemberExpression(e) => e.into_ast_kind(),
			E::StaticMemberExpression(e) => e.into_ast_kind(),
			E::PrivateFieldExpression(e) => e.into_ast_kind(),
		}
	}
}

impl<'ast> IntoAstKind<'ast> for &'ast PropertyKey<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			PropertyKey::StaticIdentifier(e) => e.into_ast_kind(),
			PropertyKey::PrivateIdentifier(e) => e.into_ast_kind(),
			PropertyKey::BooleanLiteral(e) => e.into_ast_kind(),
			PropertyKey::NullLiteral(e) => e.into_ast_kind(),
			PropertyKey::NumericLiteral(e) => e.into_ast_kind(),
			PropertyKey::BigIntLiteral(e) => e.into_ast_kind(),
			PropertyKey::RegExpLiteral(e) => e.into_ast_kind(),
			PropertyKey::StringLiteral(e) => e.into_ast_kind(),
			PropertyKey::TemplateLiteral(e) => e.into_ast_kind(),
			PropertyKey::Identifier(e) => e.into_ast_kind(),
			PropertyKey::ImportMeta(e) => e.into_ast_kind(),
			PropertyKey::NewTarget(e) => e.into_ast_kind(),
			PropertyKey::Super(e) => e.into_ast_kind(),
			PropertyKey::ArrayExpression(e) => e.into_ast_kind(),
			PropertyKey::ArrowFunctionExpression(e) => e.into_ast_kind(),
			PropertyKey::AssignmentExpression(e) => e.into_ast_kind(),
			PropertyKey::AwaitExpression(e) => e.into_ast_kind(),
			PropertyKey::BinaryExpression(e) => e.into_ast_kind(),
			PropertyKey::CallExpression(e) => e.into_ast_kind(),
			PropertyKey::ChainExpression(e) => e.into_ast_kind(),
			PropertyKey::ClassExpression(e) => e.into_ast_kind(),
			PropertyKey::ConditionalExpression(e) => e.into_ast_kind(),
			PropertyKey::FunctionExpression(e) => e.into_ast_kind(),
			PropertyKey::ImportExpression(e) => e.into_ast_kind(),
			PropertyKey::LogicalExpression(e) => e.into_ast_kind(),
			PropertyKey::NewExpression(e) => e.into_ast_kind(),
			PropertyKey::ObjectExpression(e) => e.into_ast_kind(),
			PropertyKey::ParenthesizedExpression(e) => e.into_ast_kind(),
			PropertyKey::SequenceExpression(e) => e.into_ast_kind(),
			PropertyKey::TaggedTemplateExpression(e) => e.into_ast_kind(),
			PropertyKey::ThisExpression(e) => e.into_ast_kind(),
			PropertyKey::UnaryExpression(e) => e.into_ast_kind(),
			PropertyKey::UpdateExpression(e) => e.into_ast_kind(),
			PropertyKey::YieldExpression(e) => e.into_ast_kind(),
			PropertyKey::PrivateInExpression(e) => e.into_ast_kind(),
			PropertyKey::JSXElement(e) => e.into_ast_kind(),
			PropertyKey::JSXFragment(e) => e.into_ast_kind(),
			PropertyKey::TSAsExpression(e) => e.into_ast_kind(),
			PropertyKey::TSSatisfiesExpression(e) => e.into_ast_kind(),
			PropertyKey::TSTypeAssertion(e) => e.into_ast_kind(),
			PropertyKey::TSNonNullExpression(e) => e.into_ast_kind(),
			PropertyKey::TSInstantiationExpression(e) => e.into_ast_kind(),
			PropertyKey::V8IntrinsicExpression(e) => e.into_ast_kind(),
			PropertyKey::ComputedMemberExpression(e) => e.into_ast_kind(),
			PropertyKey::StaticMemberExpression(e) => e.into_ast_kind(),
			PropertyKey::PrivateFieldExpression(e) => e.into_ast_kind(),
		}
	}
}

impl<'ast> IntoAstKind<'ast> for &'ast BindingPattern<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			BindingPattern::BindingIdentifier(e) => e.into_ast_kind(),
			BindingPattern::ObjectPattern(e) => e.into_ast_kind(),
			BindingPattern::ArrayPattern(e) => e.into_ast_kind(),
			BindingPattern::AssignmentPattern(e) => e.into_ast_kind(),
		}
	}
}

impl<'ast> IntoAstKind<'ast> for &'ast S<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			S::BlockStatement(e) => e.into_ast_kind(),
			S::BreakStatement(e) => e.into_ast_kind(),
			S::ContinueStatement(e) => e.into_ast_kind(),
			S::DebuggerStatement(e) => e.into_ast_kind(),
			S::DoWhileStatement(e) => e.into_ast_kind(),
			S::EmptyStatement(e) => e.into_ast_kind(),
			S::ExpressionStatement(e) => e.into_ast_kind(),
			S::ForInStatement(e) => e.into_ast_kind(),
			S::ForOfStatement(e) => e.into_ast_kind(),
			S::ForStatement(e) => e.into_ast_kind(),
			S::IfStatement(e) => e.into_ast_kind(),
			S::LabeledStatement(e) => e.into_ast_kind(),
			S::ReturnStatement(e) => e.into_ast_kind(),
			S::SwitchStatement(e) => e.into_ast_kind(),
			S::ThrowStatement(e) => e.into_ast_kind(),
			S::TryStatement(e) => e.into_ast_kind(),
			S::WhileStatement(e) => e.into_ast_kind(),
			S::WithStatement(e) => e.into_ast_kind(),
			S::VariableDeclaration(e) => e.into_ast_kind(),
			S::FunctionDeclaration(e) => e.into_ast_kind(),
			S::ClassDeclaration(e) => e.into_ast_kind(),
			S::TSTypeAliasDeclaration(e) => e.into_ast_kind(),
			S::TSInterfaceDeclaration(e) => e.into_ast_kind(),
			S::TSEnumDeclaration(e) => e.into_ast_kind(),
			S::TSNamespaceDeclaration(e) => e.into_ast_kind(),
			S::TSExternalModuleDeclaration(e) => e.into_ast_kind(),
			S::TSGlobalDeclaration(e) => e.into_ast_kind(),
			S::TSImportEqualsDeclaration(e) => e.into_ast_kind(),
			S::ImportDeclaration(e) => e.into_ast_kind(),
			S::ExportAllDeclaration(e) => e.into_ast_kind(),
			S::ExportDefaultDeclaration(e) => e.into_ast_kind(),
			S::ExportNamedDeclaration(e) => e.into_ast_kind(),
			S::TSExportAssignment(e) => e.into_ast_kind(),
			S::TSNamespaceExportDeclaration(e) => e.into_ast_kind(),
			S::ExportFromDeclaration(e) => e.into_ast_kind(),
			S::ExportDeclaration(e) => e.into_ast_kind(),
		}
	}
}
