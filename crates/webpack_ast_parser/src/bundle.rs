use crate::{parser::WebpackAstParser, sync::ThreadSafeParser};
use anyhow::{Result, bail};
use explorer_types::{IncomingModuleDeps, ModuleId};
use oxc::span::Span;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub enum Location {
	Path(PathBuf),
	Inline(Arc<str>),
}

/// O(1) clone
#[derive(Debug, Clone)]
pub struct Reference {
	pub location: Location,
	pub module_id: ModuleId,
	pub range: Span,
}

/// O(1) clone
#[derive(Debug, Clone)]
pub struct Definition {
	pub location: Location,
	pub module_id: ModuleId,
	pub range: Span,
}

pub trait IModuleDepProvider {
	fn get_module_deps(&self, id: ModuleId) -> Result<Arc<IncomingModuleDeps>>;
}

impl<T: IModuleDepProvider + ?Sized> IModuleDepProvider for &T {
	fn get_module_deps(&self, id: ModuleId) -> Result<Arc<IncomingModuleDeps>> {
		(**self).get_module_deps(id)
	}
}

pub trait IModuleCache {
	fn get_module_filepath(&self, id: ModuleId) -> Option<PathBuf>;
	fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>>;
	fn get_latest_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
	) -> Result<Arc<ThreadSafeParser>> {
		self.get_module_parser(requestor, id, Some(true))
	}
}

impl<T: IModuleCache + ?Sized> IModuleCache for &T {
	fn get_module_filepath(&self, id: ModuleId) -> Option<PathBuf> {
		(**self).get_module_filepath(id)
	}

	fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		(**self).get_module_parser(requestor, id, latest)
	}

	fn get_latest_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
	) -> Result<Arc<ThreadSafeParser>> {
		(**self).get_latest_module_parser(requestor, id)
	}
}

pub(crate) struct DefaultModuleDepProvider;

pub(crate) struct DefaultModuleCache;

impl IModuleDepProvider for DefaultModuleDepProvider {
	fn get_module_deps(
		&self,
		_id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		bail!("No module dependency provider provided");
	}
}

impl IModuleCache for DefaultModuleCache {
	fn get_module_filepath(&self, _id: ModuleId) -> Option<PathBuf> {
		None
	}

	fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		_id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		bail!("No module cache provided");
	}
}
