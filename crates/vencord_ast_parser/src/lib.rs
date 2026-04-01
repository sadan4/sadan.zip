mod hash;
mod patches;
mod types;
mod pass;
pub mod parser;

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
pub use parser::VencordAstParser;


#[cfg(test)]
mod tests;