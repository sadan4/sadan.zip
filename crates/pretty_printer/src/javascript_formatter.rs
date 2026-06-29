//! Last content commit was `59ef2610ba7c43fc42c64fc71f6096e91e19ae16`
//! <https://github.com/ChromeDevTools/devtools-frontend/commit/59ef2610ba7c43fc42c64fc71f6096e91e19ae16>
use std::iter::{self};

use anyhow::{Result, bail};
use oxc::{
	allocator::{Allocator, TakeIn, Vec as OxcVec},
	ast::{
		AstKind,
		Comment,
		ast::{
			ArrowFunctionExpression,
			BlockStatement,
			BreakStatement,
			ContinueStatement,
			DoWhileStatement,
			ExportNamedDeclaration,
			Expression,
			ForInStatement,
			ForOfStatement,
			ForStatement,
			Function,
			FunctionBody,
			FunctionType,
			Hashbang,
			IfStatement,
			ImportDeclaration,
			Program,
			Statement,
			StaticMemberExpression,
			TemplateLiteral,
			TryStatement,
			WhileStatement,
			WithStatement,
		},
	},
	ast_visit::Visit,
	diagnostics::Severity,
	parser::{Kind, Parser, Token, config::TokensParserConfig},
	semantic::{AstNodes, NodeId, SemanticBuilder},
	span::{GetSpan, SourceType},
};

use crate::{
	FormattedContent,
	formatted_content_builder::FormattedContentBuilder,
	javascript_formatter::token_stream::{TokenOrComment, TokenStream},
};

mod token_stream {
	use std::hint::likely;

	use derive_more::{From, IsVariant};

	use super::{Comment, GetSpan, Kind, OxcVec, Token};
	pub struct TokenStream<'a> {
		tokens: OxcVec<'a, Token>,
		comments: OxcVec<'a, Comment>,
	}

	impl<'a> TokenStream<'a> {
		pub fn new(
			mut tokens: OxcVec<'a, Token>,
			mut comments: OxcVec<'a, Comment>,
		) -> Self {
			debug_assert!(
				tokens.is_sorted_by_key(|t| t.span().start),
				"Tokens must be sorted by their start position"
			);
			debug_assert!(
				comments.is_sorted_by_key(|c| c.span.start),
				"Comments must be sorted by their start position"
			);
			// drop any EOF tokens at the end
			while tokens
				.last()
				.is_some_and(|t| t.kind() == Kind::Eof)
			{
				tokens.pop();
			}
			// reverse the tokens and comments so we can pop instead
			// of removing from the front and having to shift the elements
			tokens.reverse();
			comments.reverse();
			Self { tokens, comments }
		}
	}

	#[derive(Copy, Clone, Debug, From, IsVariant)]
	pub enum TokenOrComment {
		Token(Token),
		Comment(Comment),
	}

	impl TokenOrComment {
		pub fn start(&self) -> u32 {
			match self {
				Self::Token(token) => token.start(),
				Self::Comment(comment) => comment.span.start,
			}
		}
	}

	impl GetSpan for TokenOrComment {
		fn span(&self) -> oxc::span::Span {
			match self {
				Self::Token(token) => token.span(),
				Self::Comment(comment) => comment.span,
			}
		}
	}

	impl Iterator for TokenStream<'_> {
		type Item = TokenOrComment;

		fn next(&mut self) -> Option<Self::Item> {
			match (self.tokens.last(), self.comments.last()) {
				(Some(token), Some(comment)) => {
					debug_assert_ne!(
						token.span().start,
						comment.span.start,
						"Tokens and comments must not have the same start position"
					);
					// most programs will have more tokens than comments
					if likely(token.span().start < comment.span.start) {
						// SAFETY: we just checked that self.tokens.last() is Some
						Some(
							unsafe { self.tokens.pop().unwrap_unchecked() }
								.into(),
						)
					} else {
						// SAFETY: we just checked that self.comments.last() is Some
						Some(
							unsafe { self.comments.pop().unwrap_unchecked() }
								.into(),
						)
					}
				}
				(None, None) => None,
				(None, Some(_comment)) => Some(
					// SAFETY: comment is Some
					unsafe { self.comments.pop().unwrap_unchecked() }.into(),
				),
				(Some(_token), None) => {
					// SAFETY: token is Some
					Some(unsafe { self.tokens.pop().unwrap_unchecked() }.into())
				}
			}
		}
	}
}

pub struct JavaScriptFormatter<'a> {
	builder: FormattedContentBuilder<'a>,
	content: &'a str,
	last_line_number: u32,
	program: &'a Program<'a>,
	tokenizer: iter::Peekable<TokenStream<'a>>,
	nodes: AstNodes<'a>,
	/// index is line number, value is pos of first char in that line
	line_pos_cache: Vec<u32>,
}

#[derive(Debug)]
enum FormatDirective {
	Token,
	NewLine,
	Space,
	HardSpace,
	Indent,
	Dedent,
}

fn is_punct(tk: Kind) -> bool {
	!tk.is_number()
		&& tk != Kind::RegExp
		&& tk != Kind::String
		&& !tk.is_identifier_or_keyword()
}

fn is_kw(tk: Kind) -> bool {
	tk.is_any_keyword()
		&& tk != Kind::True
		&& tk != Kind::False
		&& tk != Kind::Null
}

const fn is_block(stmt: &Statement) -> bool {
	matches!(stmt, Statement::BlockStatement(_))
}

const fn is_if(stmt: &Statement) -> bool {
	matches!(stmt, Statement::IfStatement(_))
}

fn cache_lines(src: &str) -> Vec<u32> {
	if src.is_empty() {
		return vec![];
	}
	let mut line_pos_cache = vec![0];
	for (i, c) in src.char_indices() {
		if c == '\n' {
			line_pos_cache.push(i as u32 + 1);
		}
	}
	if *line_pos_cache.last().unwrap() >= src.len() as u32 {
		line_pos_cache.pop();
	}
	line_pos_cache
}

fn stmt_id(stmt: &Statement) -> NodeId {
	match stmt {
		Statement::BlockStatement(n) => n.node_id(),
		Statement::BreakStatement(n) => n.node_id(),
		Statement::ContinueStatement(n) => n.node_id(),
		Statement::DebuggerStatement(n) => n.node_id(),
		Statement::DoWhileStatement(n) => n.node_id(),
		Statement::EmptyStatement(n) => n.node_id(),
		Statement::ExpressionStatement(n) => n.node_id(),
		Statement::ForInStatement(n) => n.node_id(),
		Statement::ForOfStatement(n) => n.node_id(),
		Statement::ForStatement(n) => n.node_id(),
		Statement::IfStatement(n) => n.node_id(),
		Statement::LabeledStatement(n) => n.node_id(),
		Statement::ReturnStatement(n) => n.node_id(),
		Statement::SwitchStatement(n) => n.node_id(),
		Statement::ThrowStatement(n) => n.node_id(),
		Statement::TryStatement(n) => n.node_id(),
		Statement::WhileStatement(n) => n.node_id(),
		Statement::WithStatement(n) => n.node_id(),
		Statement::VariableDeclaration(n) => n.node_id(),
		Statement::FunctionDeclaration(n) => n.node_id(),
		Statement::ClassDeclaration(n) => n.node_id(),
		Statement::TSTypeAliasDeclaration(n) => n.node_id(),
		Statement::TSInterfaceDeclaration(n) => n.node_id(),
		Statement::TSEnumDeclaration(n) => n.node_id(),
		Statement::TSModuleDeclaration(n) => n.node_id(),
		Statement::TSGlobalDeclaration(n) => n.node_id(),
		Statement::TSImportEqualsDeclaration(n) => n.node_id(),
		Statement::ImportDeclaration(n) => n.node_id(),
		Statement::ExportAllDeclaration(n) => n.node_id(),
		Statement::ExportDefaultDeclaration(n) => n.node_id(),
		Statement::ExportNamedDeclaration(n) => n.node_id(),
		Statement::TSExportAssignment(n) => n.node_id(),
		Statement::TSNamespaceExportDeclaration(n) => n.node_id(),
	}
}

impl<'a> JavaScriptFormatter<'a> {
	fn is_in_for_loop_header(&self, node_id: NodeId) -> bool {
		let parent = self.nodes.parent_kind(node_id);
		matches!(
			parent,
			AstKind::ForStatement(_)
				| AstKind::ForInStatement(_)
				| AstKind::ForOfStatement(_)
		)
	}

	pub fn run(
		alloc: &'a Allocator,
		mut builder: FormattedContentBuilder<'a>,
		content: &'a str,
	) -> Result<FormattedContent> {
		let mut parsed = Parser::new(alloc, content, SourceType::default())
			.with_config(TokensParserConfig)
			.parse();
		if parsed.panicked {
			let err = parsed
				.diagnostics
				.into_iter()
				.find(|d| d.severity == Severity::Error)
				.expect("parser panicked but provided no error")
				.with_source_code(String::from(content));
			bail!("Failed to parse JavaScript content: {err:?}");
		}
		parsed
			.program
			.comments
			.sort_by_key(|c| c.span.start);
		let comments = parsed.program.comments.take_in(&alloc);
		let tokens = parsed.tokens;
		builder.hint_num_tokens(comments.len() + tokens.len());
		let tok_stream = TokenStream::new(tokens, comments);
		let (_, nodes) = SemanticBuilder::new()
			.with_build_nodes(true)
			.build(&parsed.program)
			.semantic
			.into_scoping_and_nodes();
		let mut formatter = JavaScriptFormatter::<'_> {
			builder,
			content,
			last_line_number: 0,
			program: &parsed.program,
			tokenizer: tok_stream.peekable(),
			nodes,
			line_pos_cache: cache_lines(content),
		};
		formatter.format();
		Ok(formatter.builder.build_with_mappings())
	}

	fn format(&mut self) {
		self.visit_program(self.program);
	}

	#[expect(
		clippy::cognitive_complexity,
		clippy::too_many_lines,
		reason = "TODO: refactor?"
	)]
	fn format_token(
		&self,
		node: AstKind<'_>,
		token: &TokenOrComment,
	) -> &'static [FormatDirective] {
		use AstKind as N;
		use FormatDirective as F;
		use Kind as TK;
		let token = match token {
			TokenOrComment::Token(token) => token,
			TokenOrComment::Comment(_) => return &[F::Token, F::NewLine],
		};

		let tk = token.kind();
		// tracing::trace!(
		// 	"Processing token: {:?} in node: {:?}",
		// 	tk,
		// 	node.debug_name()
		// );

		#[expect(clippy::match_same_arms)]
		match node {
			N::ContinueStatement(ContinueStatement { label, .. })
			| N::BreakStatement(BreakStatement { label, .. }) => {
				if label.is_some() && is_kw(tk) {
					&[F::Token, F::Space]
				} else {
					&[F::Token]
				}
			}
			N::BindingIdentifier(_)
			| N::IdentifierName(_)
			| N::IdentifierReference(_) => &[F::Token],
			N::PrivateIdentifier(_) => &[F::Token],
			N::ReturnStatement(n) => {
				if tk == TK::Semicolon {
					&[F::Token]
				} else if n.argument.is_some() {
					&[F::Token, F::Space]
				} else {
					&[F::Token]
				}
			}
			N::AwaitExpression(_) => {
				if tk == TK::Semicolon {
					&[F::Token]
				} else {
					&[F::Token, F::Space]
				}
			}
			N::ObjectProperty(_) if tk == TK::Colon => &[F::Token, F::Space],
			N::ArrayExpression(_) if tk == TK::Comma => &[F::Token, F::Space],
			N::LabeledStatement(_) if tk == TK::Colon => &[F::Token, F::Space],
			N::LogicalExpression(_)
			| N::AssignmentExpression(_)
			| N::BinaryExpression(_)
				if is_punct(tk) && tk != TK::RParen && tk != TK::LParen =>
			{
				&[F::Space, F::Token, F::Space]
			}
			N::ConditionalExpression(_)
				if tk == TK::Question || tk == TK::Colon =>
			{
				&[F::Space, F::Token, F::Space]
			}
			N::VariableDeclarator(_) if tk == TK::Eq => {
				&[F::Space, F::Token, F::Space]
			}
			N::ObjectPattern(_)
				if tk == TK::LCurly
					&& matches!(
						self.nodes.parent_kind(node.node_id()),
						N::VariableDeclarator(_)
					) =>
			{
				&[F::Space, F::Token]
			}
			N::ObjectPattern(_) if tk == TK::Comma => &[F::Token, F::Space],
			N::Function(n) => {
				if tk == TK::Function && n.id.is_some() {
					&[F::Token, F::Space]
				} else {
					&[F::Token]
				}
			}
			N::FormalParameters(_) | N::ArrowFunctionExpression(_)
				if tk == TK::Comma || tk == TK::RParen =>
			{
				&[F::Token, F::Space]
			}
			N::ArrowFunctionExpression(_) if tk == TK::LCurly => {
				&[F::Space, F::Token]
			}
			N::ArrowFunctionExpression(_) if tk == TK::Arrow => {
				&[F::Space, F::Token, F::Space]
			}
			N::WithStatement(n) if tk == TK::RParen => {
				if is_block(&n.body) {
					&[F::Token, F::Space]
				} else {
					&[F::Token, F::NewLine, F::Indent]
				}
			}
			N::SwitchStatement(_) if tk == TK::LCurly => {
				&[F::Token, F::NewLine, F::Indent]
			}
			N::SwitchStatement(_) if tk == TK::RCurly => {
				&[F::NewLine, F::Dedent, F::Token, F::NewLine]
			}
			N::SwitchStatement(_) if tk == TK::RParen => &[F::Token, F::Space],
			N::SwitchCase(_) if tk == TK::Case => {
				&[F::NewLine, F::Dedent, F::Token, F::Space]
			}
			N::SwitchCase(_) if tk == TK::Default => {
				&[F::NewLine, F::Dedent, F::Token]
			}
			N::SwitchCase(_) if tk == TK::Colon => {
				&[F::Token, F::NewLine, F::Indent]
			}
			N::VariableDeclaration(n) if tk == TK::Comma => {
				let all_variables_initialized = n
					.declarations
					.iter()
					.all(|decl| decl.init.is_some());
				if all_variables_initialized
					&& !self.is_in_for_loop_header(n.node_id())
				{
					&[
						F::NewLine,
						F::HardSpace,
						F::HardSpace,
						F::Token,
						F::Space,
					]
				} else {
					&[F::Token, F::Space]
				}
			}
			N::PropertyDefinition(_) if tk == TK::Eq => {
				&[F::Space, F::Token, F::Space]
			}
			N::PropertyDefinition(_) if tk == TK::Semicolon => {
				&[F::Token, F::NewLine]
			}
			N::BlockStatement(BlockStatement { body, .. })
			| N::FunctionBody(FunctionBody {
				statements: body, ..
			}) if tk == TK::LCurly && !body.is_empty() => {
				&[F::Token, F::NewLine, F::Indent]
			}
			N::BlockStatement(BlockStatement { body, .. })
			| N::FunctionBody(FunctionBody {
				statements: body, ..
			}) if tk == TK::RCurly && !body.is_empty() => {
				&[F::NewLine, F::Dedent, F::Token]
			}
			N::CatchClause(_) if tk == TK::RParen => &[F::Token, F::Space],
			N::ObjectExpression(n) if n.properties.is_empty() => &[F::Token],
			N::ObjectExpression(_) if tk == TK::LCurly => {
				&[F::Token, F::NewLine, F::Indent]
			}
			N::ObjectExpression(_) if tk == TK::RCurly => {
				&[F::NewLine, F::Dedent, F::Token]
			}
			N::ObjectExpression(_) if tk == TK::Comma => {
				&[F::Token, F::NewLine]
			}
			N::IfStatement(n) if tk == TK::RParen => {
				if is_block(&n.consequent) {
					&[F::Token, F::Space]
				} else {
					&[F::Token, F::NewLine, F::Indent]
				}
			}
			N::IfStatement(n) if tk == TK::Else => {
				if is_block(&n.consequent) {
					// st
					if n.alternate
						.as_ref()
						.is_some_and(|alt| is_block(alt) || is_if(alt))
					{
						// s
						&[F::Space, F::Token, F::Space]
					} else {
						// n>
						&[F::Space, F::Token, F::NewLine, F::Indent]
					}
				} else {
					// n<t
					if n.alternate
						.as_ref()
						.is_some_and(|alt| is_block(alt) || is_if(alt))
					{
						// s
						&[F::NewLine, F::Dedent, F::Token, F::Space]
					} else {
						// n>
						&[
							F::NewLine,
							F::Dedent,
							F::Token,
							F::NewLine,
							F::Indent,
						]
					}
				}
			}
			N::CallExpression(_) if tk == TK::Comma => &[F::Token, F::Space],
			N::SequenceExpression(n) if tk == TK::Comma => {
				if self
					.nodes
					.parent_kind(n.node_id())
					.as_switch_case()
					.is_some()
				{
					&[F::Token, F::Space]
				} else {
					&[F::Token, F::NewLine]
				}
			}
			N::ForStatement(_)
			| N::ForOfStatement(_)
			| N::ForInStatement(_)
				if tk == TK::Semicolon =>
			{
				&[F::Token, F::Space]
			}
			N::ForStatement(_)
			| N::ForOfStatement(_)
			| N::ForInStatement(_)
				if tk == TK::In || tk == TK::Of =>
			{
				&[F::Space, F::Token, F::Space]
			}
			N::ForStatement(ForStatement { body, .. })
			| N::ForOfStatement(ForOfStatement { body, .. })
			| N::ForInStatement(ForInStatement { body, .. })
			| N::WhileStatement(WhileStatement { body, .. })
				if tk == TK::RParen =>
			{
				if is_block(body) {
					&[F::Token, F::Space]
				} else {
					&[F::Token, F::NewLine, F::Indent]
				}
			}
			N::DoWhileStatement(DoWhileStatement { body, .. })
				if tk == TK::Do =>
			{
				if is_block(body) {
					&[F::Token, F::Space]
				} else {
					&[F::Token, F::NewLine, F::Indent]
				}
			}
			N::DoWhileStatement(DoWhileStatement { body, .. })
				if tk == TK::While =>
			{
				if is_block(body) {
					&[F::Space, F::Token, F::Space]
				} else {
					&[F::NewLine, F::Dedent, F::Token, F::Space]
				}
			}
			N::DoWhileStatement(_) if tk == TK::Semicolon => {
				&[F::Token, F::NewLine]
			}
			N::ClassBody(_) if tk == TK::LCurly => {
				&[F::Space, F::Token, F::NewLine, F::Indent]
			}
			N::ClassBody(_) if tk == TK::RCurly => {
				&[F::Dedent, F::NewLine, F::Token, F::NewLine]
			}
			N::YieldExpression(_) | N::Super(_) | N::ImportExpression(_) => {
				&[F::Token]
			}
			N::ExportAllDeclaration(_) if tk == TK::Star => {
				&[F::Space, F::Token, F::Space]
			}
			N::ExportNamedDeclaration(_) | N::ImportDeclaration(_)
				if tk == TK::LCurly =>
			{
				&[F::Space, F::Token]
			}
			N::ExportNamedDeclaration(_) | N::ImportDeclaration(_)
				if tk == TK::Comma =>
			{
				&[F::Token, F::Space]
			}
			N::ExportNamedDeclaration(_) | N::ImportDeclaration(_)
				if tk == TK::Star =>
			{
				&[F::Space, F::Token, F::Space]
			}
			N::ExportNamedDeclaration(ExportNamedDeclaration {
				source: Some(source),
				..
			})
			| N::ImportDeclaration(ImportDeclaration { source, .. })
				if tk == TK::RCurly =>
			{
				&[F::Token, F::Space]
			}
			N::StaticMemberExpression(StaticMemberExpression {
				object: Expression::NumericLiteral(_),
				..
			}) => &[F::Space, F::Token],

			_ if is_kw(tk) && tk != TK::This => &[F::Token, F::Space],
			_ => &[F::Token],
		}
	}

	// TODO: can this be improved by doing a binary search like thing
	fn line_of_pos(&self, pos: u32) -> u32 {
		for (line_number, first_char_pos) in self
			.line_pos_cache
			.iter()
			.enumerate()
			.rev()
		{
			if pos >= *first_char_pos {
				return line_number as u32;
			}
		}
		unreachable!()
	}

	fn push(
		&mut self,
		token: Option<&TokenOrComment>,
		format: &[FormatDirective],
	) {
		for inst in format {
			match inst {
				FormatDirective::Space => self.builder.add_soft_space(),
				FormatDirective::HardSpace => self.builder.add_hard_space(),
				FormatDirective::NewLine => self.builder.add_new_line(None),
				FormatDirective::Indent => {
					self.builder.increase_nesting_level();
				}
				FormatDirective::Dedent => {
					self.builder.decrease_nesting_level();
				}
				FormatDirective::Token => {
					if let Some(token) = token {
						let span = token.span();
						if self.line_of_pos(span.start) - self.last_line_number
							> 1
						{
							self.builder.add_new_line(Some(true));
						}
						self.last_line_number = self.line_of_pos(span.end);
						self.builder
							.add_token(&self.content[span], span.start);
					} else {
						debug_assert!(
							false,
							"Token directive requires a token"
						);
					}
				}
			}
		}
	}

	fn finish_node(&self, node: AstKind<'a>) -> &'static [FormatDirective] {
		use AstKind as N;
		use FormatDirective as F;
		match node {
			N::WithStatement(WithStatement { body, .. }) if !is_block(body) => {
				&[F::NewLine, F::Dedent]
			}
			N::VariableDeclaration(_)
				if !self.is_in_for_loop_header(node.node_id()) =>
			{
				&[F::NewLine]
			}
			N::ForStatement(ForStatement { body, .. })
			| N::ForOfStatement(ForOfStatement { body, .. })
			| N::ForInStatement(ForInStatement { body, .. })
				if !is_block(body) =>
			{
				&[F::NewLine, F::Dedent]
			}
			// Evil logic
			// TODO: refactor / clean up?
			N::BlockStatement(_) | N::FunctionBody(_) => {
				let parent = self.nodes.parent_kind(node.node_id());
				let grand_parent = self.nodes.parent_kind(parent.node_id());
				if let N::ArrowFunctionExpression(ArrowFunctionExpression {
					expression: true,
					..
				}) = parent
				{
					return &[];
				}
				if let N::IfStatement(parent) = parent
					&& parent.alternate.is_some()
					&& stmt_id(&parent.consequent) == node.node_id()
				{
					return &[];
				}
				if matches!(
					parent,
					N::Function(Function {
						r#type: FunctionType::FunctionExpression,
						..
					})
				) {
					// Function expressions only need a trailing newline when
					// they are the value of a class method definition (so the
					// next class member starts on its own line). Anywhere else
					// (call args, var initializers, object properties,
					// assignment RHS, template literal expressions, ...) the
					// surrounding expression continues on the same line.
					if !matches!(grand_parent, N::MethodDefinition(_)) {
						return &[];
					}
				}
				if let N::DoWhileStatement(_) = parent {
					return &[];
				}
				if let N::TryStatement(TryStatement { block, .. }) = parent
					&& block.node_id() == node.node_id()
				{
					return &[F::Space];
				}
				if let N::CatchClause(_) = parent
					&& let N::TryStatement(TryStatement {
						finalizer: Some(_),
						..
					}) = grand_parent
				{
					return &[F::Space];
				}
				&[F::NewLine]
			}
			N::WhileStatement(WhileStatement { body, .. })
				if !is_block(body) =>
			{
				&[F::NewLine, F::Dedent]
			}
			N::IfStatement(IfStatement {
				alternate: Some(stmt),
				..
			}) if !is_block(stmt) && !is_if(stmt) => &[F::Dedent],
			N::IfStatement(IfStatement {
				consequent: stmt, ..
			}) if !is_block(stmt) => &[F::Dedent],
			// Arrow functions with a single expression have an expression statement as their body
			N::ExpressionStatement(_)
				if self
					.nodes
					.parent_kind(node.node_id())
					.as_function_body()
					.is_some_and(|fb| {
						let gp = self.nodes.parent_kind(fb.node_id());
						gp.as_arrow_function_expression()
							.is_some_and(|gp| gp.expression)
					}) =>
			{
				&[]
			}
			N::BreakStatement(_)
			| N::ContinueStatement(_)
			| N::ThrowStatement(_)
			| N::ReturnStatement(_)
			| N::ExpressionStatement(_)
			| N::ImportDeclaration(_)
			| N::ExportAllDeclaration(_)
			| N::ExportDefaultDeclaration(_)
			| N::ExportNamedDeclaration(_) => &[F::NewLine],
			_ => &[],
		}
	}
}

impl<'a> Visit<'a> for JavaScriptFormatter<'a> {
	fn enter_node(&mut self, kind: AstKind<'a>) {
		let node_start = kind.span().start;
		while let Some(token) = self.tokenizer.peek()
			&& token.start() < node_start
		{
			// SAFETY: we just peeked and the next token is Some
			let token = unsafe { self.tokenizer.next().unwrap_unchecked() };
			let format = self
				.format_token(self.nodes.parent_kind(kind.node_id()), &token);

			self.push(Some(&token), format);
		}
	}

	fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
		let it = self.alloc(it);
		self.builder
			.set_enforce_space_between_words(false);
		self.enter_node(AstKind::TemplateLiteral(it));
		self.visit_span(&it.span);
		debug_assert!(it.expressions.len() + 1 == it.quasis.len(), "how?");
		let mut quasis = it.quasis.iter();
		// visit the quasis in source order
		// oxc will visit all quasis then all elements
		// instead of visiting in source order
		self.visit_template_element(quasis.next().unwrap());
		for (expr, quasi) in it.expressions.iter().zip(&it.quasis) {
			// Expressions inside ${...} are real JavaScript and need normal
			// inter-word spacing (e.g. `foo instanceof bar`, `async function`).
			self.builder
				.set_enforce_space_between_words(true);
			self.visit_expression(expr);
			self.builder
				.set_enforce_space_between_words(false);
			self.visit_template_element(quasi);
		}
		self.leave_node(AstKind::TemplateLiteral(it));
		self.builder
			.set_enforce_space_between_words(true);
	}

	fn leave_node(&mut self, kind: AstKind<'a>) {
		let restore = self
			.builder
			.set_enforce_space_between_words(
				kind.as_template_element().is_none()
					&& kind.as_template_literal().is_none(),
			);
		let node_end = kind.span().end;
		while let Some(token) = self.tokenizer.peek()
			&& token.start() < node_end
		{
			// SAFETY: we just peeked and the next token is Some
			let token = unsafe { self.tokenizer.next().unwrap_unchecked() };
			let format = self.format_token(kind, &token);

			self.push(Some(&token), format);
		}
		self.push(None, self.finish_node(kind));
		self.builder
			.set_enforce_space_between_words(restore);
	}

	/// Oxc is bugged and doesn't provide a hashbang token
	/// despite having a [token type for it](Kind::HashbangComment)
	fn visit_hashbang(&mut self, it: &Hashbang<'a>) {
		self.builder
			.add_token(&self.content[it.span], it.span.start);
		self.builder.add_new_line(None);
	}

	/// handle program as a special case so we don't have to branch
	///  on it in [`Self::enter_node`] and [`Self::leave_node`]
	fn visit_program(&mut self, it: &Program<'a>) {
		if let Some(hashbang) = &it.hashbang {
			self.visit_hashbang(hashbang);
		}
		self.visit_directives(&it.directives);
		self.visit_statements(&it.body);
		// finish any trailing comments
		while let Some(token) = self.tokenizer.next() {
			debug_assert!(
				token.is_comment(),
				"Expected only comments after program end"
			);
			let format = self.format_token(AstKind::Program(it), &token);
			self.push(Some(&token), format);
		}
	}
}
