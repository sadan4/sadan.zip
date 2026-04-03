use crate::exts::ModuleDeclarationExt;
use anyhow::{Result, bail};
use itertools::Itertools;
use oxc::{
	allocator::{Allocator, Box as OxcBox},
	ast::{
		AstKind,
		ast::{ImportDeclaration, ModuleDeclaration, Program},
	},
	parser::Parser as OxcParser,
	semantic::{
		AstNode, NodeId, Reference, Scoping, Semantic, SemanticBuilder,
		SymbolId,
	},
	span::SourceType,
};
use std::sync::Arc;

macro_rules! impl_parse {
	($alloc:expr, $source:expr, $source_type:expr, $ast:ident, $sema:ident) => {
		let parsed = OxcParser::new($alloc, $source, $source_type).parse();
		if parsed.panicked {
			let dbg_src = Arc::new($source.to_string());
			let errs_with_src = parsed
				.errors
				.into_iter()
				.map(move |err| err.with_source_code(dbg_src.clone()))
				.collect_vec();
			bail!(
				"OxcParser panicked while parsing source. errors: \n{:?}\n",
				errs_with_src
			);
		}
		if !parsed.errors.is_empty() {
			let dbg_src = Arc::new($source.to_string());
			let errs_with_src = parsed
				.errors
				.into_iter()
				.map(move |err| err.with_source_code(dbg_src.clone()))
				.collect_vec();
			bail!("Failed to parse source: \n{:#?}\n", errs_with_src);
		}
		let $ast: &'ast mut Program<'ast> = $alloc.alloc(parsed.program);
		let $sema = SemanticBuilder::new()
			.with_cfg(true)
			.with_check_syntax_error(true)
			.build($ast);
		if !$sema.errors.is_empty() {
			let dbg_src = Arc::new($source.to_string());
			let errs_with_src = $sema
				.errors
				.into_iter()
				.map(move |err| err.with_source_code(dbg_src.clone()))
				.collect_vec();
			bail!(
				"Failed to perform semantic analysis on source: \n{:#?}\n",
				errs_with_src
			);
		}
	};
}

pub fn parse<'ast>(
	alloc: &'ast Allocator,
	source: &'ast str,
	source_type: SourceType,
) -> Result<(&'ast Program<'ast>, Semantic<'ast>)> {
	impl_parse!(alloc, source, source_type, ast, sema);
	Ok((ast, sema.semantic))
}

pub fn parse_no_sema<'ast>(
	alloc: &'ast Allocator,
	source: &'ast str,
	source_type: SourceType,
) -> Result<Program<'ast>> {
	let parsed = OxcParser::new(alloc, source, source_type).parse();
	if parsed.panicked {
		let dbg_src = Arc::new(source.to_string());
		let errs_with_src = parsed
			.errors
			.into_iter()
			.map(move |err| err.with_source_code(dbg_src.clone()))
			.collect_vec();
		bail!(
			"OxcParser panicked while parsing source. errors: \n{errs_with_src:?}\n"
		);
	}
	if !parsed.errors.is_empty() {
		let dbg_src = Arc::new(source.to_string());
		let errs_with_src = parsed
			.errors
			.into_iter()
			.map(move |err| err.with_source_code(dbg_src.clone()))
			.collect_vec();
		bail!("Failed to parse source: \n{errs_with_src:#?}\n");
	}
	Ok(parsed.program)
}

pub fn parse_for_traverse<'ast>(
	alloc: &'ast Allocator,
	source: &'ast str,
	source_type: SourceType,
) -> Result<(&'ast mut Program<'ast>, Scoping)> {
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
	fn p(&self, node_id: NodeId) -> AstKind<'ast>
	{
		self.sema().nodes().parent_node(node_id).kind()
	}

	/// Parent of node, if it matches the predicate
	/// TODO: add example
	fn p_if<T, F: FnOnce(AstKind<'ast>) -> Option<T>>(&self, node_id: NodeId, pred: F) -> Option<T> {
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
