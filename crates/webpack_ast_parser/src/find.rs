use oxc::{parser::Token, span::Span};

/// A scored find sequence.
///
/// created by [`WebpackAstParser::generate_finds`]
pub struct ScoredFindSequence {
	/// the quality of the find
	///
	/// higher is better
	pub score: u32,
	/// the tokens involved in the find
	///
	/// will be in source order, but not necessarily contiguous
	///
	/// eg: `void 0` has a gap between the token `void` and the token `0`
	pub tokens: Vec<Token>,
}

impl ScoredFindSequence {
	/// Gets the string of the find given the source the find is for
	///
	/// if `source` is from a different find, the result is unspecified and may panic
	pub fn get_find<'a>(&self, source: &'a str) -> &'a str {
		&source[Span::new(
			self.tokens[0].span().start,
			self.tokens[self.tokens.len() - 1]
				.span()
				.end,
		)]
	}
}
