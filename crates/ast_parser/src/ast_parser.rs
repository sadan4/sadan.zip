use crate::{
	exts::{ExpressionExt, ModuleDeclarationExt},
	sym_id::GetSymId,
};
use anyhow::Result;
use itertools::Itertools;
use oxc::{
	allocator::{Allocator, Box as OxcBox},
	ast::{
		AstKind,
		ast::{Expression, ImportDeclaration, ModuleDeclaration, Program},
	},
	ast_visit::Visit,
	parser::{ParseOptions, Parser as OxcParser},
	semantic::{
		AstNode,
		NodeId,
		Reference,
		Scoping,
		Semantic,
		SemanticBuilder,
		SymbolId,
	},
	span::{GetSpan as _, SourceType},
};
use std::sync::Arc;

macro_rules! impl_parse {
	($alloc:expr, $source:expr, $source_type:expr, $ast:ident, $sema:ident) => {
		let mut parsed = OxcParser::new($alloc, $source, $source_type)
			.with_options(ParseOptions {
				parse_regular_expression: true,
				..Default::default()
			})
			.parse();
		if !parsed.errors.is_empty() {
			let dbg_src = Arc::new($source.to_string());
			let err = parsed
				.errors
				.swap_remove(0)
				.with_source_code(dbg_src);
			return Err(err);
		}
		let $ast: &'ast mut Program<'ast> = $alloc.alloc(parsed.program);
		let $sema = SemanticBuilder::new()
			.with_cfg(true)
			.with_check_syntax_error(true)
			.build($ast);
		if !$sema.errors.is_empty() {
			let mut sema = $sema;
			let dbg_src = Arc::new($source.to_string());
			let err = sema
				.errors
				.swap_remove(0)
				.with_source_code(dbg_src);
			return Err(err);
		}
	};
}

pub fn parse<'ast>(
	alloc: &'ast Allocator,
	source: &'ast str,
	source_type: SourceType,
) -> Result<(&'ast Program<'ast>, Semantic<'ast>), miette::Error> {
	impl_parse!(alloc, source, source_type, ast, sema);
	Ok((ast, sema.semantic))
}

pub fn parse_no_sema<'ast>(
	alloc: &'ast Allocator,
	source: &'ast str,
	source_type: SourceType,
) -> Result<Program<'ast>, miette::Error> {
	let mut parsed = OxcParser::new(alloc, source, source_type).parse();
	if !parsed.errors.is_empty() {
		let dbg_src = Arc::new(source.to_string());
		let err = parsed
			.errors
			.swap_remove(0)
			.with_source_code(dbg_src);
		return Err(err);
	}
	Ok(parsed.program)
}

pub fn parse_for_traverse<'ast>(
	alloc: &'ast Allocator,
	source: &'ast str,
	source_type: SourceType,
) -> Result<(&'ast mut Program<'ast>, Scoping), miette::Error> {
	impl_parse!(alloc, source, source_type, ast, sema);
	let scoping = sema.semantic.into_scoping();
	Ok((ast, scoping))
}
pub trait AstParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast>;
	fn sema(&self) -> &Semantic<'ast>;
	// /// node from id
	fn n<'a>(&'a self, node_id: NodeId) -> &'a AstNode<'ast>
	where
		'ast: 'a,
	{
		self.sema().nodes().get_node(node_id)
	}
	/// Parent of node
	fn p(&self, node_id: NodeId) -> AstKind<'ast> {
		self.sema()
			.nodes()
			.parent_node(node_id)
			.kind()
	}

	/// Parent of node, if it matches the predicate
	/// TODO: add example
	fn p_if<T, F: FnOnce(AstKind<'ast>) -> Option<T>>(
		&self,
		node_id: NodeId,
		pred: F,
	) -> Option<T> {
		pred(self.p(node_id))
	}
	// fn cfg_id(&self, node_id: NodeId) -> BlockNodeId {
	//     self.sema().nodes().cfg_id(node_id)
	// }
	// fn cfg<'a: 'ast>(&'a self) -> &'ast ControlFlowGraph {
	//     // we always parse with the cfg
	//     self.sema().cfg().unwrap()
	// }
	// fn dbg_cfg<'a: 'ast>(&'a self, node_id: NodeId) -> String {
	//     let cfg_id = self.cfg_id(node_id);
	//     let cfg = self.cfg();
	//     let block = cfg.basic_block(cfg_id);
	//     let ctx = DebugDotContext::new(self.sema().nodes(), true);
	//     block.debug_dot(ctx)
	// }
	fn refs<'a>(&'a self, sym_id: SymbolId) -> impl Iterator<Item = NodeId> + 'a
	where
		'ast: 'a,
	{
		self.sema()
			.scoping()
			.get_resolved_references(sym_id)
			.map(Reference::node_id)
	}
	fn ref_nodes<'a>(
		&'a self,
		sym_id: SymbolId,
	) -> impl Iterator<Item = AstKind<'ast>> + 'a
	where
		'ast: 'a,
	{
		self.refs(sym_id)
			.map(|node_id| self.n(node_id).kind())
	}
	fn find_parent<'a, T>(
		&'a self,
		mut node_id: NodeId,
		pred: impl Fn(AstKind<'ast>) -> Option<T>,
	) -> Option<T>
	where
		'ast: 'a,
	{
		loop {
			let parent = self.p(node_id);
			if let Some(found) = pred(parent) {
				return Some(found);
			}

			let parent_id = parent.node_id();

			if parent_id == node_id {
				return None;
			}

			node_id = parent_id;
		}
	}
	fn find_parent_limited<'a, T>(
		&'a self,
		mut node_id: NodeId,
		pred: impl Fn(AstKind<'ast>) -> Option<T>,
		mut limit: usize,
	) -> Option<T>
	where
		'ast: 'a,
	{
		debug_assert!(limit > 0, "Limit must be greater than 0");
		loop {
			if limit == 0 {
				return None;
			}
			let parent = self.p(node_id);
			if let Some(found) = pred(parent) {
				return Some(found);
			}

			let parent_id = parent.node_id();

			if parent_id == node_id {
				return None;
			}

			node_id = parent_id;
			limit -= 1;
		}
	}
	// TODO: Test this method, it's a bit cursed
	fn last_parent<T>(
		&self,
		node_id: NodeId,
		pred: impl Fn(AstKind<'ast>) -> Option<T>,
	) -> Option<T> {
		let mut node = self.n(node_id).kind();
		loop {
			let parent = self.p(node.node_id());
			if parent.node_id() == node.node_id() {
				break;
			}
			if pred(parent).is_none() {
				break;
			}
			node = parent;
		}
		pred(node)
	}
	/// Compare two references to variables
	/// Returns true if they refer to the same variable
	/// Does not consider redeclarations / aliases
	fn cmp_sym(&self, a: &impl GetSymId, b: &impl GetSymId) -> bool {
		self.sym_id_of(a)
			.is_some_and(|a| Some(a) == self.sym_id_of(b))
	}

	/// Get the symbol id of anything implementing [`GetSymId`]
	fn sym_id_of(&self, of: &impl GetSymId) -> Option<SymbolId> {
		of.get_sym_id(self.sema())
	}
	// TODO: probably be better to use a small vec here since we symbolid is small and we rarely have one let alone more
	/// Given some code like
	/// ```js
	/// const bar = "foo";
	/// const baz = bar;
	/// const qux = baz;
	/// ```
	/// if given the symbol id for `qux`, this will return the symbol ids `[baz, bar]`
	fn unwrap_variable_declarator(&self, symbol_id: SymbolId) -> Vec<SymbolId> {
		let mut ret = Vec::new();
		let mut last = symbol_id;
		loop {
			let decl_id = self
				.sema()
				.scoping()
				.symbol_declaration(last);
			let Some(decl) = self
				.n(decl_id)
				.kind()
				.as_variable_declarator()
			else {
				break;
			};
			let Some(init) = &decl
				.init
				.as_ref()
				.and_then(Expression::as_identifier)
			else {
				break;
			};
			// init is a double ref, but i can't deref in the some binding. Weird.
			let Some(init_sym_id) = self.sym_id_of(*init) else {
				break;
			};
			last = init_sym_id;
			ret.push(last);
		}
		ret
	}
	/// Returns the most specific AST node whose span contains `pos`.
	fn get_node_at(&self, pos: u32) -> AstKind<'ast> {
		get_node_at(self.prog(), pos)
	}
}

pub trait ESModuleParser<'ast>: AstParser<'ast> {
	fn import_statements<'a: 'ast>(
		&'a self,
	) -> impl Iterator<Item = &'ast ImportDeclaration<'ast>> {
		self.prog()
			.body
			.iter()
			.filter_map(|node| {
				node.as_module_declaration()
					.and_then(ModuleDeclaration::as_import_declaration)
					.map(OxcBox::as_ref)
			})
	}
	fn find_import_by_name<'a: 'ast, 'b>(
		&'a self,
		from: &'b str,
	) -> Option<&'ast ImportDeclaration<'ast>> {
		let pred = |import: &&ImportDeclaration| import.source.value == from;
		debug_assert!(
			self.import_statements()
				.filter(pred)
				.count() <= 1,
			"Found multiple import statements with the same source"
		);
		// Imports can only be at the top level
		self.import_statements().find(pred)
	}
}

fn get_node_at<'ast>(prog: &'ast Program<'ast>, pos: u32) -> AstKind<'ast> {
	let mut finder = LocationFinder {
		pos,
		best: None,
		best_span_size: u32::MAX,
	};
	finder.visit_program(prog);
	finder
		.best
		.unwrap_or(AstKind::Program(prog))
}

struct LocationFinder<'ast> {
	pos: u32,
	best: Option<AstKind<'ast>>,
	best_span_size: u32,
}

impl<'ast> Visit<'ast> for LocationFinder<'ast> {
	fn enter_node(&mut self, kind: AstKind<'ast>) {
		let span = kind.span();
		let contains_pos = (span.start..span.end).contains(&self.pos);
		if !contains_pos {
			return;
		}

		let span_size = span.size();
		if span_size <= self.best_span_size {
			self.best_span_size = span_size;
			self.best = Some(kind);
		}
	}
}
