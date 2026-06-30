pub mod diag;
pub mod hash;
pub mod parser;
mod pass;
pub mod patches;
mod types;

pub use oxc::allocator::Allocator;
pub use parser::VencordAstParser;
pub use types::{
	Dev,
	Match,
	MatchLike,
	MatchRegex,
	Patch,
	PluginDev,
	PluginInfo,
	ReplaceLike,
	Replacement,
	Replacer,
	TemplateEvaluator,
	find::{AnyFindType, FindArg, FindData, FindType, FindUse},
};

#[cfg(test)]
mod tests;
