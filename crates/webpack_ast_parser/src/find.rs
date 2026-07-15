use oxc::{parser::Token, span::Span};
use smol_str::SmolStr;

/// An i18n/intl key referenced within a find sequence.
pub struct IntlKey {
	/// the 6-char hashed key as it appears in source (eg `Go5Vvs`)
	pub hashed: SmolStr,
	/// the original, unhashed message name (eg `ADD_TO_FAVOURITES`) when it
	/// could be resolved from the embedded key mapping; `None` otherwise
	pub unhashed: Option<SmolStr>,
}

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
	/// all i18n/intl keys referenced within the find sequence
	///
	/// in source order; may contain duplicates
	pub intl_keys: Vec<IntlKey>,
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
