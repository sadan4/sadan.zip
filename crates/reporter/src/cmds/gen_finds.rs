use miette::Result;
use explorer_types::ModuleId;
use crate::util::{MultiProgressWrapper, generate_unique_finds};
use crate::Cli;
use crate::fetcher::fetch_build;

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

    let src = modules.get(&module_id).unwrap();
    for find in finds {
        println!("Score: {}\n{}\n", find.score, find.get_find(src));
    }

    Ok(())
}
