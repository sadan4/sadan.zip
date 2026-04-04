#![warn(missing_docs)]
mod ast_parser;
pub mod exts;
pub mod sym_id;
pub mod ast_kind;

pub use ast_parser::{AstParser, ESModuleParser, parse, parse_no_sema, parse_for_traverse};
