use std::pin::Pin;

use explorer_types::ModuleId;
use oxc::allocator::Allocator;
use tower_lsp::async_trait;
use webpack_ast_parser::WebpackAstParser;

struct SplitModuleCache {
	disk: Box<dyn ModuleCache>,
	live: Box<dyn ModuleCache>,
}
#[async_trait]
trait ModuleCache: Send + Sync + 'static {
}
