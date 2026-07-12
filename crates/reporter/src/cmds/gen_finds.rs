use crate::{
	Cli,
	fetcher::fetch_build,
	util::{MultiProgressWrapper, generate_unique_finds},
};
use explorer_types::ModuleId;
use miette::Result;
use webpack_ast_parser::WebpackAstParser;

pub async fn gen_finds(
	module_id: ModuleId,
	cli: &Cli,
	global_bar: &MultiProgressWrapper,
) -> Result<()> {
	let builds = fetch_build(cli.fetch_opts.clone(), global_bar)
		.await
		.map_err(|e| miette::miette!("Failed to fetch build: {:?}", e))?;

	if builds.is_empty() {
		miette::bail!("No builds fetched");
	}

	let modules = &builds[0].modules;
	let finds = generate_unique_finds(module_id, modules, global_bar)?;

	let mut src = modules.get(&module_id).unwrap().clone();
	if !WebpackAstParser::is_webpack_module(&src) {
		WebpackAstParser::format_module_header(&mut src, module_id, false);
	}
	for find in finds {
		println!("Score: {}\n{}\n", find.score, find.get_find(&src));
	}

	Ok(())
}
