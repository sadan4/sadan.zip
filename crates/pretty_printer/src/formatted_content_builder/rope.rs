use oxc::allocator::{Allocator, Vec as OxcVec};

#[derive(Debug)]
pub struct Rope<'s> {
	strs: OxcVec<'s, &'s str>,
	total_len: usize,
}

impl<'s> Rope<'s> {
	pub fn new_in(alloc: &'s Allocator) -> Self {
		Self {
			strs: OxcVec::new_in(&alloc),
			total_len: 0,
		}
	}

	pub fn last_char(&self) -> Option<char> {
		self.strs
			.last()
			.and_then(|s| s.chars().last())
	}

	pub const fn is_empty(&self) -> bool {
		self.total_len == 0
	}

	pub const fn len(&self) -> usize {
		self.total_len
	}

	#[expect(clippy::inherent_to_string)]
	pub fn to_string(&self) -> String {
		let mut result = String::new();
		result.reserve_exact(self.total_len);
		for s in &self.strs {
			result.push_str(s);
		}
		debug_assert_eq!(result.len(), self.total_len);
		result
	}

	pub fn push(&mut self, s: &'s str) {
		self.total_len += s.len();
		self.strs.push(s);
	}

	pub fn reserve(&mut self, additional: usize) {
		#[expect(
			clippy::cast_precision_loss,
			reason = "we just want a rough estimate here"
		)]
		self.strs
			.reserve((additional as f64 * 1.75) as usize);
	}
}
