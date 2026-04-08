use crate::{parser::WebpackAstParser, types::ModuleId};
use anyhow::{Result, bail};
use oxc::span::Span;
use smol_str::SmolStr;
use std::rc::Rc;

/// Information about a module's dependents
#[derive(Debug, Clone, Default)]
pub struct IncomingModuleDeps {
	/// The modules that require this module synchronously
	pub sync: Vec<ModuleId>,
	/// the module that require this module lazily (dynamic import)
	pub lazy: Vec<ModuleId>,
}

/// Information about a module's dependencies
#[derive(Default, Clone, Debug)]
pub struct OutgoingModuleDeps {
	/// The modules that this module requires synchronously
	pub sync: Vec<ModuleId>,
	/// the module that this module requires lazily (dynamic import)
	pub lazy: Vec<ModuleId>,
}

#[derive(Debug, Clone)]
pub enum Location<'a> {
	Path(SmolStr),
	Inline(&'a str),
}

/// O(1) clone
#[derive(Debug, Clone)]
pub struct Reference<'a> {
	pub location: Location<'a>,
	pub module_id: ModuleId,
	pub range: Span,
}

/// O(1) clone
#[derive(Debug, Clone)]
pub struct Definition<'a> {
	pub location: Location<'a>,
	pub module_id: ModuleId,
	pub range: Span,
}

pub trait IModuleDepProvider {
	fn get_module_deps(&self, id: ModuleId) -> Result<Rc<IncomingModuleDeps>>;
}

pub trait IModuleCache<'ast> {
	fn get_module_filepath(&self, id: ModuleId) -> Option<SmolStr>;
	fn get_module_parser(
		&self,
		requestor: &WebpackAstParser<'ast>,
		id: ModuleId,
		latest: Option<bool>,
	) -> Result<Rc<WebpackAstParser<'ast>>>;
	fn get_latest_module_parser(
		&self,
		requestor: &WebpackAstParser<'ast>,
		id: ModuleId,
	) -> Result<Rc<WebpackAstParser<'ast>>> {
		self.get_module_parser(requestor, id, Some(true))
	}
}

pub(crate) struct DefaultModuleDepProvider;

pub(crate) struct DefaultModuleCache;

impl IModuleDepProvider for DefaultModuleDepProvider {
	fn get_module_deps(&self, _id: ModuleId) -> Result<Rc<IncomingModuleDeps>> {
		bail!("No module dependency provider provided");
	}
}

impl<'ast> IModuleCache<'ast> for DefaultModuleCache {
	fn get_module_filepath(&self, _id: ModuleId) -> Option<SmolStr> {
		None
	}

	fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'ast>,
		_id: ModuleId,
		_latest: Option<bool>,
	) -> Result<Rc<WebpackAstParser<'ast>>> {
		bail!("No module cache provided");
	}
}
