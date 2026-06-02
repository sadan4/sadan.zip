use bitflags::bitflags;
use oxc::{
	allocator::Vec as OxcVec,
	regular_expression::{
		ast::{CapturingGroup, IndexedReference, NamedReference, Pattern},
		visit::{
			Visit as VisitRegex,
			walk::{
				walk_capturing_group,
				walk_indexed_reference,
				walk_named_reference,
			},
		},
	},
	span::{GetSpan, Span},
};

#[derive(Debug, Clone, Copy)]
pub enum GroupReference<'ast> {
	Indexed(&'ast IndexedReference),
	Named(&'ast NamedReference<'ast>),
}

impl GetSpan for GroupReference<'_> {
	fn span(&self) -> Span {
		match self {
			GroupReference::Indexed(IndexedReference { span, .. })
			| GroupReference::Named(NamedReference { span, .. }) => *span,
		}
	}
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub struct CaptureFlags: u8 {
		const DUPLICATE_NAMED = 1;
	}
}

#[derive(Debug)]
pub struct Capture<'ast> {
	pub group: &'ast CapturingGroup<'ast>,
	pub refs: OxcVec<'ast, GroupReference<'ast>>,
	pub flags: CaptureFlags,
}

#[derive(Debug)]
pub struct GroupInfo<'ast> {
	pub indexed_groups: OxcVec<'ast, Capture<'ast>>,
	pub unbound_refs: OxcVec<'ast, GroupReference<'ast>>,
}

pub fn collect_capture_groups<'ast>(
	parser: &super::VencordAstParser<'ast>,
	pat: &'ast Pattern<'ast>,
) -> GroupInfo<'ast> {
	let mut v = CapturingGroups {
		info: GroupInfo {
			indexed_groups: OxcVec::new_in(parser.alloc),
			unbound_refs: OxcVec::new_in(parser.alloc),
		},
		parser,
	};
	v.visit_pattern(pat);
	v.info
}

struct CapturingGroups<'ast, 'p> {
	parser: &'p super::VencordAstParser<'ast>,
	info: GroupInfo<'ast>,
}
// TODO: warn on `\0` it's null, not a backreference
impl<'ast> VisitRegex<'ast> for CapturingGroups<'ast, '_> {
	fn visit_capturing_group(&mut self, it: &CapturingGroup<'ast>) {
		let it = self.alloc(it);
		// check that it has not been previously declared
		let mut gr = Capture {
			group: it,
			refs: OxcVec::new_in(self.parser.alloc),
			flags: CaptureFlags::empty(),
		};
		if let Some(name) = it.name
			&& self
				.info
				.indexed_groups
				.iter()
				.any(|c| c.group.name == Some(name))
		{
			gr.flags |= CaptureFlags::DUPLICATE_NAMED;
			// let diag = ParserDiagnostic {
			// 	msg: "Duplicate named capturing group".into(),
			// 	main_span: it.span,
			// 	labels: vec![(
			// 		orig_group.span,
			// 		format!("Previous declaration of group `{name}`").into(),
			// 	)],
			// 	..Default::default()
			// };
			// diag_ch.send(diag).ok();
		}
		self.info.indexed_groups.push(gr);
		walk_capturing_group(self, it);
	}
	fn visit_indexed_reference(&mut self, it: &IndexedReference) {
		let it = self.alloc(it);
		if let Some(group) = self
			.info
			.indexed_groups
			.get_mut((it.index - 1) as usize)
		{
			group
				.refs
				.push(GroupReference::Indexed(it));
		} else {
			self.info
				.unbound_refs
				.push(GroupReference::Indexed(it));
		}
		walk_indexed_reference(self, it);
	}
	fn visit_named_reference(&mut self, it: &NamedReference<'ast>) {
		let it = self.alloc(it);
		if let Some(group) = self
			.info
			.indexed_groups
			.iter_mut()
			.find(|g| g.group.name == Some(it.name))
		{
			group
				.refs
				.push(GroupReference::Named(it));
		} else {
			self.info
				.unbound_refs
				.push(GroupReference::Named(it));
		}
		walk_named_reference(self, it);
	}
}
