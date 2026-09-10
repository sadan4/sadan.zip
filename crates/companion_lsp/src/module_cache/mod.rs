use std::{fmt::Write as _, fs, path::PathBuf, pin::Pin, sync::Arc};

use dashmap::DashMap;
use explorer_types::ModuleId;
use oxc::allocator::Allocator;
use smol_str::format_smolstr;
use tower_lsp::async_trait;
use tracing::{instrument, warn};
use webpack_ast_parser::{
	ThreadSafeParser,
	WebpackAstParser,
	bundle::{IModuleCache, IModuleDepProvider},
};

struct SplitModuleCache {
	disk: Box<dyn ModuleCache>,
	live: Box<dyn ModuleCache>,
}

trait ModuleCache:
	IModuleCache + IModuleDepProvider + Send + Sync + 'static
{
}

impl<T> ModuleCache for T where
	T: IModuleCache + IModuleDepProvider + Send + Sync + 'static + ?Sized
{
}

struct DiskModuleCache {
	parsers: DashMap<ModuleId, Arc<ThreadSafeParser>>,
	module_root: PathBuf,
}

impl IModuleCache for DiskModuleCache {
	#[instrument(skip_all, fields(id))]
	fn get_module_filepath(&self, id: ModuleId) -> Option<PathBuf> {
		let mut path = self.module_root.clone();
		// module id 6 + ext 3 + `/` 1
		path.reserve(10);
		path.set_trailing_sep(true);
		write!(path.as_mut_os_string(), "{id}.js").unwrap();
		match fs::exists(&path) {
			Ok(e) => e.then_some(path),
			Err(e) => {
				warn!(path =% path.display(), "Failed to check existance of module file: {e}");
				None
			}
		}
	}

	fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		latest: Option<bool>,
	) -> anyhow::Result<Arc<ThreadSafeParser>> {
		todo!()
	}
}
