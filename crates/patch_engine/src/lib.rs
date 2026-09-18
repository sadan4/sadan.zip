//! Apply a [`Patch`](vencord_ast_parser::Patch) to the source of a webpack
//! module, the same way Vencord does at runtime.

mod apply;
mod compile;
mod fmt;

pub use apply::{
	Applied,
	ApplyEvent,
	ApplyOptions,
	SyntaxErrorReport,
	apply_patch,
	check_syntax_errors,
	matches_module,
};
pub use compile::compile_patch_regexes;
pub use fmt::format_syntax_error;

#[cfg(test)]
mod tests;
