use crate::{parser::WebpackAstParser, types::ModuleId};
use anyhow::{Result, bail};
use std::path::PathBuf;

pub trait IModuleCache<'ast> {
	fn get_module_filepath(&self, id: ModuleId) -> Option<PathBuf>;
	fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'ast>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<&'ast WebpackAstParser<'ast>>;
	fn get_latest_module_parser(
		&self,
		requestor: &WebpackAstParser<'ast>,
		id: ModuleId,
	) -> Result<&'ast WebpackAstParser<'ast>> {
		self.get_module_parser(requestor, id, Some(true))
	}
}

pub(crate) struct DefaultModuleCache;

impl<'ast> IModuleCache<'ast> for DefaultModuleCache {
	fn get_module_filepath(&self, _id: ModuleId) -> Option<PathBuf> {
		None
	}

	fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'ast>,
		_id: ModuleId,
		_latest: Option<bool>,
	) -> Result<&'ast WebpackAstParser<'ast>> {
		bail!("No module cache provided");
	}
}
