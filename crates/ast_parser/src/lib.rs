mod ast_parser;
pub mod exts;

pub use ast_parser::{AstParser, ESModuleParser, parse, parse_no_sema, parse_for_traverse};
