mod hash;
pub mod parser;
mod pass;
mod patches;
mod types;

pub use parser::VencordAstParser;
pub use types::{
	Match,
	MatchLike,
	MatchRegex,
	Patch,
	ReplaceLike,
	Replacement,
	Replacer,
	TemplateEvaluator,
};

#[cfg(test)]
mod tests;
