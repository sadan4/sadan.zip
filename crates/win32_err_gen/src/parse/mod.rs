pub mod std_page;
pub mod tbl_page;

use anyhow::{Context as _, Result, anyhow};
use markdown::{ParseOptions, mdast, to_mdast};

pub async fn parse_doc_page(url: &str) -> Result<mdast::Node> {
	let res = reqwest::get(url)
		.await
		.with_context(|| format!("Failed to fetch doc page: {url}"))?;
	let text = res
		.text()
		.await
		.with_context(|| format!("Failed to read doc page: {url}"))?;
	let root_node = to_mdast(&text, &ParseOptions::gfm())
		.map_err(|err| anyhow!(err))
		.with_context(|| format!("Failed to parse doc page: {url}"))?;
	Ok(root_node)
}

pub async fn dump_ast(url: &str) -> Result<()> {
	let ast = parse_doc_page(url).await?;
	println!("{ast:#?}");
	Ok(())
}
