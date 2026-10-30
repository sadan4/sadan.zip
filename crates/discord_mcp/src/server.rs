//! The MCP server: tool parameters, descriptions and dispatch

use std::{fmt::Write as _, sync::Arc};

use bundle_source::explorer::ExplorerClient;
use rmcp::{
	ErrorData,
	RoleServer,
	ServerHandler,
	handler::server::{tool::ToolRouter, wrapper::Parameters},
	model::{
		Implementation,
		ListResourceTemplatesResult,
		PaginatedRequestParams,
		ReadResourceRequestParams,
		ReadResourceResponse,
		ReadResourceResult,
		ResourceContents,
		ResourceTemplate,
		ServerCapabilities,
		ServerConfig,
	},
	schemars::JsonSchema,
	service::RequestContext,
	tool,
	tool_handler,
	tool_router,
};
use serde::Deserialize;
use tracing::info;

use crate::{
	progress::McpProgress,
	source::{self, BuildSource},
	store::{BundleStore, LoadedBundle, parse_module_id},
	tools::{
		css::{self, CssTarget},
		exports,
		intl,
		misc,
		modules::{self, DepDirection},
		patch::{self, PluginInput},
	},
};

type ToolResult = Result<String, String>;

fn done(r: anyhow::Result<String>) -> ToolResult {
	r.map_err(|e| format!("{e:#}"))
}

const INSTRUCTIONS: &str = "\
Tools for writing Discord client mods (Vencord and similar) against Discord's \
webpack bundle. Everything works offline on a downloaded build; nothing talks \
to a running Discord client.

Start with load_build (the newest archived build is \
{\"source\": {\"kind\": \"explorer\"}}). Every other tool works on the most \
recently loaded build unless given `build` (a build hash, unique hash prefix, \
or \"stable\"/\"canary\").

Typical patch workflow: search_modules or find_module to locate code, \
get_module to read it (line numbers in every tool refer to the pretty printed \
module), generate_finds for a unique find, then test_patch to apply a plugin's \
patches and see the diff or errors. intl_lookup / intl_search translate \
#{intl::KEY} strings. module_exports, find_export_uses and find_store need \
the whole build parsed, which takes a while the first time per build.

Discord's css: css_search finds rules by selector, declaration or any text, \
css_variables lists custom properties with their values per theme, and \
get_css reads a whole chunk. css_classes maps a css module's exported names \
to the generated class names those rules use.";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildArg {
	/// Build to use: a build hash, unique prefix, `stable` or `canary`.
	/// Defaults to the most recently loaded build.
	#[serde(default)]
	pub build: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBuildsParams {
	/// Only builds first seen before this unix timestamp in milliseconds.
	/// Pass `next_cursor` from a previous call to page.
	#[serde(default)]
	pub before: Option<u64>,
	/// Max builds to list (default 20)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoadBuildParams {
	pub source: BuildSource,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetModuleParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Webpack module id
	pub id: String,
	/// Pretty print the module (default true). Unformatted modules are
	/// usually one huge line.
	#[serde(default)]
	pub pretty: Option<bool>,
	/// First line to return, 1-based (default 1)
	#[serde(default)]
	pub start_line: Option<usize>,
	/// Last line to return, inclusive (default `start_line` + 299)
	#[serde(default)]
	pub end_line: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Text to search for in the raw (minified) module sources. Supports
	/// `#{intl::KEY}`.
	pub query: String,
	/// Treat query as a JS regex. `\i` matches an identifier, like in Vencord
	/// patches.
	#[serde(default)]
	pub regex: bool,
	/// Max modules to list (default 20)
	#[serde(default)]
	pub limit: Option<usize>,
	/// Bytes of context around each match (default 80)
	#[serde(default)]
	pub context: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindModuleParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// A Vencord patch find: a plain string, or a regex written as
	/// `/pattern/flags`. Supports `\i` and `#{intl::KEY}`.
	pub find: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModuleIdParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Webpack module id
	pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateFindsParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Webpack module id
	pub id: String,
	/// Max finds to return (default 5)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModuleDepsParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Webpack module id
	pub id: String,
	/// `out` for modules this one requires, `in` for modules that require
	/// it, `both` (default)
	#[serde(default)]
	pub direction: DepDirection,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportUsesParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Webpack module id that defines the export
	pub id: String,
	/// Path to the export, eg `["Z"]` or `["default", "getUser"]`. `default` is
	/// the default export / module.exports.
	pub export_path: Vec<String>,
	/// Max uses to list (default 50)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Case insensitive substring to look for. Empty lists everything.
	#[serde(default)]
	pub query: String,
	/// Max results (default 25)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CssSearchParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Text to find, eg a class name like `chatContent` or `chatContent_f75fb0`
	pub query: String,
	/// Treat query as a JS regex
	#[serde(default)]
	pub regex: bool,
	/// Where to look: `any` (default), `selector` or `declaration`
	#[serde(default)]
	pub target: CssTarget,
	/// Max rules to show (default 20)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CssVariablesParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Part of the variable name, eg `background-primary`. Empty lists all.
	#[serde(default)]
	pub query: String,
	/// Max variables to show (default 30)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCssParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Chunk name (or unique prefix) from `css_search` results. Omit to
	/// list every chunk.
	#[serde(default)]
	pub chunk: Option<String>,
	/// First line to return, 1-based (default 1)
	#[serde(default)]
	pub start_line: Option<usize>,
	/// Last line to return, inclusive (default `start_line` + 299)
	#[serde(default)]
	pub end_line: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindStoreParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Store display name or part of it, eg `UserStore`
	pub name: String,
	/// Max stores to list (default 20)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiffModuleParams {
	/// Webpack module id
	pub id: String,
	/// Older build (hash, prefix or channel)
	pub from: String,
	/// Newer build (hash, prefix or channel)
	pub to: String,
	/// Unchanged lines around each change (default 3)
	#[serde(default)]
	pub context: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IntlLookupParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Key name (`USER_SETTINGS`), `#{intl::USER_SETTINGS}`, or a 6 character
	/// hashed key
	pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PluginParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Path to the plugin's entry file (`index.ts` or `index.tsx`)
	#[serde(default)]
	pub plugin_path: Option<String>,
	/// Plugin source, instead of `plugin_path`. Use it to test edits before
	/// writing them.
	#[serde(default)]
	pub plugin_source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestPatchParams {
	#[serde(flatten)]
	pub plugin: PluginParams,
	/// Only test the patch at this index in the plugin's patches array
	#[serde(default)]
	pub patch_index: Option<usize>,
	/// Max modules to apply each patch to when its find matches several
	/// (default 2)
	#[serde(default)]
	pub max_modules: Option<usize>,
	/// Unchanged lines around each change in diffs (default 3)
	#[serde(default)]
	pub context: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckPluginsParams {
	#[serde(flatten)]
	pub build: BuildArg,
	/// Root of a Vencord (or fork) checkout
	pub vencord_dir: String,
	/// Plugin dirs relative to `vencord_dir`. Defaults to `src/plugins`,
	/// `src/plugins/_api` and `src/plugins/_core`; `src/userplugins` must be
	/// passed explicitly.
	#[serde(default)]
	pub plugin_dirs: Vec<String>,
	/// Also list warnings, like ambiguous finds (default false)
	#[serde(default)]
	pub include_warnings: bool,
	/// Max problems to list (default 30)
	#[serde(default)]
	pub limit: Option<usize>,
}

#[derive(Clone)]
pub struct DiscordMcp {
	store: Arc<BundleStore>,
	explorer: ExplorerClient,
	tool_router: ToolRouter<Self>,
}

#[expect(
	clippy::multiple_inherent_impl,
	reason = "#[tool_router] needs an impl of only tools"
)]
impl DiscordMcp {
	pub fn new(explorer: ExplorerClient) -> Self {
		Self {
			store: Arc::new(BundleStore::default()),
			explorer,
			tool_router: Self::tool_router(),
		}
	}

	fn bundle(&self, build: &BuildArg) -> Result<Arc<LoadedBundle>, String> {
		self.store
			.get(build.build.as_deref())
			.map_err(|e| format!("{e:#}"))
	}
}

#[tool_router]
impl DiscordMcp {
	#[tool(
		description = "List Discord builds archived by the explorer server, newest first. Use a hash with load_build.",
		annotations(read_only_hint = true)
	)]
	async fn list_builds(
		&self,
		Parameters(p): Parameters<ListBuildsParams>,
	) -> ToolResult {
		let page = self
			.explorer
			.list_builds(p.before, Some(p.limit.unwrap_or(20)))
			.await
			.map_err(|e| format!("{e:#}"))?;
		let mut out = String::new();
		for b in &page.builds {
			writeln!(
				out,
				"{} #{} {:?} first seen {} (unix ms)",
				b.build_hash, b.build_number, b.channel, b.first_seen
			)
			.unwrap();
		}
		if let Some(cursor) = page.next_cursor {
			write!(out, "\nnext_cursor: {cursor}").unwrap();
		}
		Ok(out)
	}

	#[tool(
		description = "Load a Discord build and make it the active one. Sources: explorer (archived builds, fast, default newest), scrape (live discord.com stable/canary), local (a .modules dir from the companion LSP). The 2 most recent builds stay loaded."
	)]
	async fn load_build(
		&self,
		Parameters(p): Parameters<LoadBuildParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let progress = McpProgress::for_request(&ctx);
		let bundle = source::load(p.source, &self.explorer, progress)
			.await
			.map_err(|e| format!("{e:#}"))?;
		let summary = bundle.summary();
		info!("Loaded {summary}");
		let evicted = self.store.insert(Arc::new(bundle));
		let mut out = format!("Loaded {summary}. It is now the active build.");
		if !evicted.is_empty() {
			write!(out, "\nUnloaded: {}", evicted.join(", ")).unwrap();
		}
		Ok(out)
	}

	#[tool(
		description = "List the loaded builds, the active one first.",
		annotations(read_only_hint = true)
	)]
	async fn build_info(&self) -> ToolResult {
		let all = self.store.all();
		if all.is_empty() {
			return Ok(String::from("No build is loaded. Call load_build."));
		}
		Ok(all
			.iter()
			.enumerate()
			.map(|(i, b)| {
				let active = if i == 0 { " (active)" } else { "" };
				format!("{}{active}", b.summary())
			})
			.collect::<Vec<_>>()
			.join("\n"))
	}

	#[tool(
		description = "Read a webpack module's source, pretty printed with line numbers, a page at a time.",
		annotations(read_only_hint = true)
	)]
	async fn get_module(
		&self,
		Parameters(p): Parameters<GetModuleParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(parse_module_id(&p.id).and_then(|id| {
			modules::get_module(
				&b,
				id,
				p.pretty.unwrap_or(true),
				p.start_line,
				p.end_line,
			)
		}))
	}

	#[tool(
		description = "Search every module's raw source for a string or regex. Returns module ids with a snippet around the first match.",
		annotations(read_only_hint = true)
	)]
	async fn search_modules(
		&self,
		Parameters(p): Parameters<SearchParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(modules::search_modules(
			&b,
			&p.query,
			p.regex,
			p.limit.unwrap_or(20),
			p.context.unwrap_or(80),
		))
	}

	#[tool(
		description = "Check which modules a Vencord patch find matches, and whether it is unique.",
		annotations(read_only_hint = true)
	)]
	async fn find_module(
		&self,
		Parameters(p): Parameters<FindModuleParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(modules::find_module(&b, &p.find))
	}

	#[tool(
		description = "Generate short find strings that match only the given module, best first, using #{intl::KEY} where possible.",
		annotations(read_only_hint = true)
	)]
	async fn generate_finds(
		&self,
		Parameters(p): Parameters<GenerateFindsParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		let id = parse_module_id(&p.id).map_err(|e| format!("{e:#}"))?;
		done(modules::generate_finds(b, id, p.limit.unwrap_or(5)).await)
	}

	#[tool(
		description = "List the modules a module requires and the modules that require it.",
		annotations(read_only_hint = true)
	)]
	async fn module_deps(
		&self,
		Parameters(p): Parameters<ModuleDepsParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		let id = parse_module_id(&p.id).map_err(|e| format!("{e:#}"))?;
		done(modules::module_deps(b, id, p.direction).await)
	}

	#[tool(
		description = "Show a module's exports as a tree with line numbers, including flux stores and the events they handle. Parses the whole build on first use.",
		annotations(read_only_hint = true)
	)]
	async fn module_exports(
		&self,
		Parameters(p): Parameters<ModuleIdParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		let id = parse_module_id(&p.id).map_err(|e| format!("{e:#}"))?;
		done(
			exports::module_exports(b, id, McpProgress::for_request(&ctx))
				.await,
		)
	}

	#[tool(
		description = "Find every use of a module's export across the build, following re-exports. Parses the whole build on first use.",
		annotations(read_only_hint = true)
	)]
	async fn find_export_uses(
		&self,
		Parameters(p): Parameters<ExportUsesParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		let id = parse_module_id(&p.id).map_err(|e| format!("{e:#}"))?;
		done(
			exports::find_export_uses(
				b,
				id,
				&p.export_path,
				p.limit.unwrap_or(50),
				McpProgress::for_request(&ctx),
			)
			.await,
		)
	}

	#[tool(
		description = "Find flux stores by display name, with their module, export path and handled flux events. Parses the whole build on first use.",
		annotations(read_only_hint = true)
	)]
	async fn find_store(
		&self,
		Parameters(p): Parameters<FindStoreParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			exports::find_store(
				b,
				&p.name,
				p.limit.unwrap_or(20),
				McpProgress::for_request(&ctx),
			)
			.await,
		)
	}

	#[tool(
		description = "Diff one module between two loaded builds, to see why a patch broke. Load both builds first.",
		annotations(read_only_hint = true)
	)]
	async fn diff_module(
		&self,
		Parameters(p): Parameters<DiffModuleParams>,
	) -> ToolResult {
		let from = self.bundle(&BuildArg {
			build: Some(p.from),
		})?;
		let to = self.bundle(&BuildArg { build: Some(p.to) })?;
		done(parse_module_id(&p.id).and_then(|id| {
			modules::diff_module(&from, &to, id, p.context.unwrap_or(3))
		}))
	}

	#[tool(
		description = "Look up an intl message by key name, #{intl::KEY} or hash. Returns its text and the #{intl::...} form to use in patches.",
		annotations(read_only_hint = true)
	)]
	async fn intl_lookup(
		&self,
		Parameters(p): Parameters<IntlLookupParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(intl::intl_lookup(b, &p.key, McpProgress::for_request(&ctx)).await)
	}

	#[tool(
		description = "Search intl messages by English text or key name. Returns keys usable as #{intl::KEY} in finds and patches.",
		annotations(read_only_hint = true)
	)]
	async fn intl_search(
		&self,
		Parameters(p): Parameters<QueryParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			intl::intl_search(
				b,
				&p.query,
				p.limit.unwrap_or(25),
				McpProgress::for_request(&ctx),
			)
			.await,
		)
	}

	#[tool(
		description = "Search the experiments defined in the build by name or contents.",
		annotations(read_only_hint = true)
	)]
	async fn list_experiments(
		&self,
		Parameters(p): Parameters<QueryParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			misc::list_experiments(
				b,
				&p.query,
				p.limit.unwrap_or(25),
				McpProgress::for_request(&ctx),
			)
			.await,
		)
	}

	#[tool(
		description = "Search css module classes by exported name or generated class name, eg to find the module for findCssClassesLazy. Parses the whole build on first use.",
		annotations(read_only_hint = true)
	)]
	async fn css_classes(
		&self,
		Parameters(p): Parameters<QueryParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			misc::css_classes(
				b,
				&p.query,
				p.limit.unwrap_or(25),
				McpProgress::for_request(&ctx),
			)
			.await,
		)
	}

	#[tool(
		description = "Search Discord's css. Each match is shown as the whole rule it is in, with its chunk, line and enclosing @media/@container. Use target=selector to find every rule for a class.",
		annotations(read_only_hint = true)
	)]
	async fn css_search(
		&self,
		Parameters(p): Parameters<CssSearchParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			css::css_search(
				b,
				&p.query,
				p.regex,
				p.target,
				p.limit.unwrap_or(20),
			)
			.await,
		)
	}

	#[tool(
		description = "List css custom properties (--variables) matching a name, with how often each is used and its value in every rule that defines it, eg per theme.",
		annotations(read_only_hint = true)
	)]
	async fn css_variables(
		&self,
		Parameters(p): Parameters<CssVariablesParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(css::css_variables(b, &p.query, p.limit.unwrap_or(30)).await)
	}

	#[tool(
		description = "Read a css chunk, formatted with line numbers, a page at a time. Without a chunk, lists every css chunk in the build.",
		annotations(read_only_hint = true)
	)]
	async fn get_css(
		&self,
		Parameters(p): Parameters<GetCssParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			css::get_css(b, p.chunk.as_deref(), p.start_line, p.end_line)
				.await,
		)
	}

	#[tool(
		description = "Search Discord's REST API endpoints table (Endpoints.NAME) by name or path.",
		annotations(read_only_hint = true)
	)]
	async fn endpoints(
		&self,
		Parameters(p): Parameters<QueryParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(misc::endpoints(b, &p.query, p.limit.unwrap_or(25)).await)
	}

	#[tool(
		description = "Apply a plugin's patches to the build the way Vencord does. Reports finds that match nothing or too much, matches that fail, and replacements that produce invalid JS, with a diff of each patched module.",
		annotations(read_only_hint = true)
	)]
	async fn test_patch(
		&self,
		Parameters(p): Parameters<TestPatchParams>,
	) -> ToolResult {
		let b = self.bundle(&p.plugin.build)?;
		done(
			patch::test_patch(
				b,
				PluginInput {
					path: p.plugin.plugin_path,
					source: p.plugin.plugin_source,
				},
				p.patch_index,
				p.max_modules.unwrap_or(2),
				p.context.unwrap_or(3),
			)
			.await,
		)
	}

	#[tool(
		description = "Check a plugin's webpack finds (findByProps, findByCode, findStore, findCssClasses, ...) against the build: broken, unique or ambiguous.",
		annotations(read_only_hint = true)
	)]
	async fn test_find(
		&self,
		Parameters(p): Parameters<PluginParams>,
		ctx: RequestContext<RoleServer>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			patch::test_find(
				b,
				PluginInput {
					path: p.plugin_path,
					source: p.plugin_source,
				},
				McpProgress::for_request(&ctx),
			)
			.await,
		)
	}

	#[tool(
		description = "Run every patch of every plugin in a Vencord checkout against the build, like the reporter, and list what is broken.",
		annotations(read_only_hint = true)
	)]
	async fn check_plugins(
		&self,
		Parameters(p): Parameters<CheckPluginsParams>,
	) -> ToolResult {
		let b = self.bundle(&p.build)?;
		done(
			patch::check_plugins(
				b,
				p.vencord_dir,
				p.plugin_dirs,
				p.include_warnings,
				p.limit.unwrap_or(30),
			)
			.await,
		)
	}
}

const MODULE_URI_PREFIX: &str = "discord://";

/// Parses `discord://<build>/module/<id>`
fn parse_module_uri(uri: &str) -> Option<(&str, &str)> {
	let rest = uri.strip_prefix(MODULE_URI_PREFIX)?;
	let (build, id) = rest.split_once("/module/")?;
	Some((build, id))
}

#[expect(
	clippy::unused_async_trait_impl,
	reason = "#[tool_handler] generates async fns too"
)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for DiscordMcp {
	fn get_info(&self) -> ServerConfig {
		ServerConfig::new(
			ServerCapabilities::builder()
				.enable_tools()
				.enable_resources()
				.build(),
		)
		.with_server_info(Implementation::new(
			"discord-mcp",
			env!("CARGO_PKG_VERSION"),
		))
		.with_instructions(INSTRUCTIONS)
	}

	async fn list_resource_templates(
		&self,
		_request: Option<PaginatedRequestParams>,
		_context: RequestContext<RoleServer>,
	) -> Result<ListResourceTemplatesResult, ErrorData> {
		Ok(ListResourceTemplatesResult::with_all_items(vec![
			ResourceTemplate::new(
				format!("{MODULE_URI_PREFIX}{{build}}/module/{{id}}"),
				"webpack module",
			)
			.with_description(
				"A webpack module of a loaded build, pretty printed. build is \
				 a loaded build's hash.",
			),
		]))
	}

	async fn read_resource(
		&self,
		request: ReadResourceRequestParams,
		_context: RequestContext<RoleServer>,
	) -> Result<ReadResourceResponse, ErrorData> {
		let uri = request.uri;
		let Some((build, id)) = parse_module_uri(&uri) else {
			return Err(ErrorData::resource_not_found(
				format!("Unknown resource {uri}"),
				None,
			));
		};
		let code = self
			.store
			.get(Some(build))
			.and_then(|b| b.formatted(parse_module_id(id)?))
			.map_err(|e| {
				ErrorData::resource_not_found(format!("{e:#}"), None)
			})?;
		Ok(ReadResourceResult::new(vec![ResourceContents::text(
			code.code.clone(),
			uri,
		)])
		.into())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_module_uris() {
		assert_eq!(
			parse_module_uri("discord://abc123/module/42"),
			Some(("abc123", "42"))
		);
		assert_eq!(parse_module_uri("discord://abc123/other/42"), None);
		assert_eq!(parse_module_uri("file:///x"), None);
	}

	#[test]
	fn tools_are_registered() {
		let server = DiscordMcp::new(ExplorerClient::default());
		let names = server
			.tool_router
			.list_all()
			.into_iter()
			.map(|t| t.name.into_owned())
			.collect::<Vec<_>>();
		assert_eq!(names.len(), 23, "{names:?}");
		assert!(names.contains(&String::from("test_patch")));
	}
}
