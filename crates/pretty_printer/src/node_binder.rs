use derive_more::Deref;
use oxc::{ast::{AstKind, ast::{Class, Program}}, ast_visit::Visit, semantic::{AstNodes, NodeFlags, NodeId, ScopeId, Stats}};

pub struct NodeBinder<'a> {
	nodes: AstNodes<'a>,
	current_node_id: NodeId,
	program: &'a Program<'a>,
}

#[derive(Deref)]
pub struct BoundNodes<'a> {
	#[deref]
	nodes: AstNodes<'a>,
	program: &'a Program<'a>,
}

impl<'a> NodeBinder<'a> {
	pub fn new(program: &'a Program<'a>) -> Self {
		let ret = Self {
			program,
			nodes: AstNodes::default(),
			current_node_id: NodeId::DUMMY,
		};
		ret
	}
	pub fn bind_nodes(mut self) -> BoundNodes<'a> {
		let stats = Stats::count(self.program);
		self.nodes.reserve(stats.nodes as usize);
		self.visit_program(self.program);
		debug_assert!(stats.nodes == self.nodes.len() as u32, "nodes count mismatch");
		BoundNodes {
			program: self.program,
			nodes: self.nodes,
		}
	}
}

const DUMMY_SCOPE_ID: ScopeId = ScopeId::new(0);
const DUMMY_NODE_FLAGS: NodeFlags = NodeFlags::empty();

impl<'a> Visit<'a> for NodeBinder<'a> {
	fn enter_node(&mut self, kind: AstKind<'a>) {
		self.current_node_id = self.nodes.add_node(
			kind,
			DUMMY_SCOPE_ID,
			self.current_node_id,
			DUMMY_NODE_FLAGS,
		)
	}

	fn visit_program(&mut self, it: &Program<'a>) {
		let kind = AstKind::Program(self.alloc(it));

		self.current_node_id = self.nodes.add_program_node(kind, DUMMY_SCOPE_ID, DUMMY_NODE_FLAGS);

		if let Some(hashbang) = &it.hashbang {
			self.visit_hashbang(hashbang);
		}
		for directive in &it.directives {
			self.visit_directive(directive);
		}

		self.visit_statements(&it.body);
	}
}
