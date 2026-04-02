pub mod base;
mod types;
mod lazy_chunk_parser;
mod main_chunk_parser;

pub(crate) trait Sealed {}

pub use lazy_chunk_parser::WebpackLazyChunkParser;
pub use main_chunk_parser::WebpackMainChunkParser;
pub use types::*;
