use oxc::{
	ast::{
		AstKind,
		ast::{
			BindingPattern,
			Expression,
			MemberExpression,
			PropertyKey,
			Statement,
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
make_impl!(TSModuleDeclaration);
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

impl<'ast> IntoAstKind<'ast> for &'ast Expression<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			Expression::BooleanLiteral(e) => e.into_ast_kind(),
			Expression::NullLiteral(e) => e.into_ast_kind(),
			Expression::NumericLiteral(e) => e.into_ast_kind(),
			Expression::BigIntLiteral(e) => e.into_ast_kind(),
			Expression::RegExpLiteral(e) => e.into_ast_kind(),
			Expression::StringLiteral(e) => e.into_ast_kind(),
			Expression::TemplateLiteral(e) => e.into_ast_kind(),
			Expression::Identifier(e) => e.into_ast_kind(),
			Expression::ImportMeta(e) => e.into_ast_kind(),
			Expression::NewTarget(e) => e.into_ast_kind(),
			Expression::Super(e) => e.into_ast_kind(),
			Expression::ArrayExpression(e) => e.into_ast_kind(),
			Expression::ArrowFunctionExpression(e) => e.into_ast_kind(),
			Expression::AssignmentExpression(e) => e.into_ast_kind(),
			Expression::AwaitExpression(e) => e.into_ast_kind(),
			Expression::BinaryExpression(e) => e.into_ast_kind(),
			Expression::CallExpression(e) => e.into_ast_kind(),
			Expression::ChainExpression(e) => e.into_ast_kind(),
			Expression::ClassExpression(e) => e.into_ast_kind(),
			Expression::ConditionalExpression(e) => e.into_ast_kind(),
			Expression::FunctionExpression(e) => e.into_ast_kind(),
			Expression::ImportExpression(e) => e.into_ast_kind(),
			Expression::LogicalExpression(e) => e.into_ast_kind(),
			Expression::NewExpression(e) => e.into_ast_kind(),
			Expression::ObjectExpression(e) => e.into_ast_kind(),
			Expression::ParenthesizedExpression(e) => e.into_ast_kind(),
			Expression::SequenceExpression(e) => e.into_ast_kind(),
			Expression::TaggedTemplateExpression(e) => e.into_ast_kind(),
			Expression::ThisExpression(e) => e.into_ast_kind(),
			Expression::UnaryExpression(e) => e.into_ast_kind(),
			Expression::UpdateExpression(e) => e.into_ast_kind(),
			Expression::YieldExpression(e) => e.into_ast_kind(),
			Expression::PrivateInExpression(e) => e.into_ast_kind(),
			Expression::JSXElement(e) => e.into_ast_kind(),
			Expression::JSXFragment(e) => e.into_ast_kind(),
			Expression::TSAsExpression(e) => e.into_ast_kind(),
			Expression::TSSatisfiesExpression(e) => e.into_ast_kind(),
			Expression::TSTypeAssertion(e) => e.into_ast_kind(),
			Expression::TSNonNullExpression(e) => e.into_ast_kind(),
			Expression::TSInstantiationExpression(e) => e.into_ast_kind(),
			Expression::V8IntrinsicExpression(e) => e.into_ast_kind(),
			Expression::ComputedMemberExpression(e) => e.into_ast_kind(),
			Expression::StaticMemberExpression(e) => e.into_ast_kind(),
			Expression::PrivateFieldExpression(e) => e.into_ast_kind(),
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

impl<'ast> IntoAstKind<'ast> for &'ast Statement<'ast> {
	fn into_ast_kind(self) -> AstKind<'ast> {
		match self {
			Statement::BlockStatement(block_statement) => {
				block_statement.into_ast_kind()
			}
			Statement::BreakStatement(e) => e.into_ast_kind(),
			Statement::ContinueStatement(e) => e.into_ast_kind(),
			Statement::DebuggerStatement(e) => e.into_ast_kind(),
			Statement::DoWhileStatement(e) => e.into_ast_kind(),
			Statement::EmptyStatement(e) => e.into_ast_kind(),
			Statement::ExpressionStatement(e) => e.into_ast_kind(),
			Statement::ForInStatement(e) => e.into_ast_kind(),
			Statement::ForOfStatement(e) => e.into_ast_kind(),
			Statement::ForStatement(e) => e.into_ast_kind(),
			Statement::IfStatement(e) => e.into_ast_kind(),
			Statement::LabeledStatement(e) => e.into_ast_kind(),
			Statement::ReturnStatement(e) => e.into_ast_kind(),
			Statement::SwitchStatement(e) => e.into_ast_kind(),
			Statement::ThrowStatement(e) => e.into_ast_kind(),
			Statement::TryStatement(e) => e.into_ast_kind(),
			Statement::WhileStatement(e) => e.into_ast_kind(),
			Statement::WithStatement(e) => e.into_ast_kind(),
			Statement::VariableDeclaration(e) => e.into_ast_kind(),
			Statement::FunctionDeclaration(e) => e.into_ast_kind(),
			Statement::ClassDeclaration(e) => e.into_ast_kind(),
			Statement::TSTypeAliasDeclaration(e) => e.into_ast_kind(),
			Statement::TSInterfaceDeclaration(e) => e.into_ast_kind(),
			Statement::TSEnumDeclaration(e) => e.into_ast_kind(),
			Statement::TSModuleDeclaration(e) => e.into_ast_kind(),
			Statement::TSGlobalDeclaration(e) => e.into_ast_kind(),
			Statement::TSImportEqualsDeclaration(e) => e.into_ast_kind(),
			Statement::ImportDeclaration(e) => e.into_ast_kind(),
			Statement::ExportAllDeclaration(e) => e.into_ast_kind(),
			Statement::ExportDefaultDeclaration(e) => e.into_ast_kind(),
			Statement::ExportNamedDeclaration(e) => e.into_ast_kind(),
			Statement::TSExportAssignment(e) => e.into_ast_kind(),
			Statement::TSNamespaceExportDeclaration(e) => e.into_ast_kind(),
			Statement::ExportFromDeclaration(e) => e.into_ast_kind(),
			Statement::ExportDeclaration(e) => e.into_ast_kind(),
		}
	}
}
