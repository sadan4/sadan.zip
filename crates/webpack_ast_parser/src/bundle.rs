use crate::{parser::WebpackAstParser, sync::ThreadSafeParser};
use anyhow::{Result, bail};
use async_trait::async_trait;
use explorer_types::{IncomingModuleDeps, ModuleId};
use oxc::span::Span;
use url::Url;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Location {
	Path(Url),
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
	/// The id of the module that contains this definition.
	pub module_id: ModuleId,
	pub range: Span,
}

#[async_trait]
pub trait IModuleDepProvider: Send + Sync {
	async fn get_module_deps(&self, id: ModuleId) -> Result<Arc<IncomingModuleDeps>>;
}

#[async_trait]
impl<T: IModuleDepProvider + ?Sized> IModuleDepProvider for &T {
	async fn get_module_deps(&self, id: ModuleId) -> Result<Arc<IncomingModuleDeps>> {
		(**self).get_module_deps(id).await
	}
}

#[async_trait]
pub trait IModuleCache: Send + Sync {
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url>;
	async fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>>;
	async fn get_latest_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
	) -> Result<Arc<ThreadSafeParser>> {
		self.get_module_parser(requestor, id, Some(true)).await
	}
}

#[async_trait]
impl<T: IModuleCache + ?Sized> IModuleCache for &T {
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url> {
		(**self).get_module_filepath(id).await
	}

	async fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		(**self).get_module_parser(requestor, id, latest).await
	}

	async fn get_latest_module_parser(
		&self,
		requestor: &WebpackAstParser<'_>,
		id: ModuleId,
	) -> Result<Arc<ThreadSafeParser>> {
		(**self).get_latest_module_parser(requestor, id).await
	}
}

pub(crate) struct DefaultModuleDepProvider;

pub(crate) struct DefaultModuleCache;

#[async_trait]
impl IModuleDepProvider for DefaultModuleDepProvider {
	async fn get_module_deps(
		&self,
		_id: ModuleId,
	) -> Result<Arc<IncomingModuleDeps>> {
		bail!("No module dependency provider provided");
	}
}

#[async_trait]
impl IModuleCache for DefaultModuleCache {
	async fn get_module_filepath(&self, _id: ModuleId) -> Option<Url> {
		None
	}

	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		_id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Arc<ThreadSafeParser>> {
		bail!("No module cache provided");
	}
}
