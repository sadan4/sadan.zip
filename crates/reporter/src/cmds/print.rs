use std::io::{self, Write as _};

use explorer_types::ModuleId;
use miette::{Context as _, IntoDiagnostic as _, Result, miette};
use webpack_ast_parser::WebpackAstParser;

use crate::fetcher::http::fetch_full_bundle;

/// Pretty print the source of `module_id` from the build with `build_hash`
/// to stdout.
pub async fn print_module(
	build_hash: &str,
	module_id: ModuleId,
	indent: u8,
) -> Result<()> {
	let mut bundle = fetch_full_bundle(build_hash)
		.await
		.map_err(|e| {
			miette!("{e:?}").context(format!(
				"Failed to fetch bundle for build {build_hash}"
			))
		})?;
	let mut src = bundle
		.modules
		.remove(&module_id)
		.ok_or_else(|| {
			miette!("Module {module_id} not found in build {build_hash}")
		})?;
	if !WebpackAstParser::is_webpack_module(&src) {
		WebpackAstParser::format_module_header(&mut src, module_id, false);
	}
	let formatted = pretty_printer::format_to_str(&src, indent)
		.map_err(|e| miette!("{e:?}"))
		.with_context(|| format!("Failed to format module {module_id}"))?;
	let mut stdout = io::stdout().lock();
	stdout
		.write_all(formatted.as_bytes())
		.and_then(|()| stdout.write_all(b"\n"))
		.into_diagnostic()
		.context("Failed to write to stdout")
}
