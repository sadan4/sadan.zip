use anyhow::Context;
use itertools::Itertools;
use tracing::error;
use tracing_subscriber::EnvFilter;
use win32_err_gen::{parse_doc_page, std_page, tbl_page};

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env())
		.init();
	let std_pages = [
		"desktop-src/Debug/system-error-codes--0-499-.md",
		"desktop-src/Debug/system-error-codes--500-999-.md",
		"desktop-src/Debug/system-error-codes--1000-1299-.md",
		"desktop-src/Debug/system-error-codes--12000-15999-.md",
		"desktop-src/Debug/system-error-codes--1300-1699-.md",
		"desktop-src/Debug/system-error-codes--1700-3999-.md",
		"desktop-src/Debug/system-error-codes--4000-5999-.md",
		"desktop-src/Debug/system-error-codes--6000-8199-.md",
		"desktop-src/Debug/system-error-codes--8200-8999-.md",
		"desktop-src/Debug/system-error-codes--9000-11999-.md",
	].into_iter()
	.map(|path| format!("https://raw.githubusercontent.com/MicrosoftDocs/win32/refs/heads/docs/{path}"));
	let mut all_codes = Vec::new();
	let mut handles = std_pages
		.map(|url| {
			tokio::spawn(async move {
				let ast = parse_doc_page(&url)
					.await
					.context("Failed to get doc ast")?;
				let codes = match std_page::parse(&ast) {
					Ok(codes) => codes,
					Err(err) => {
						error!("Failed to parse {url}: {err:#?}");
						return Err(err);
					}
				};
				Ok(codes)
			})
		})
		// we need to collect the handles so they all spawn before we start awaiting them
		.collect_vec();
	handles.push(tokio::spawn(async {
		// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-erref/705fb797-2175-4a90-b5a3-3918024b10b8
		const URL: &str = "https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-erref/705fb797-2175-4a90-b5a3-3918024b10b8?accept=text%2Fmarkdown";
		let ast = parse_doc_page(URL).await.context("Failed to get doc ast")?;
		let codes = match tbl_page::parse(ast){ 
			Ok(codes) => codes,
			Err(err) => {
				error!("Failed to parse {URL}: {err:#?}");
				return Err(err);
			}
		};
		Ok(codes)
	}));
	for handle in handles {
		let codes = match handle
			.await
			.context("Join Error")
			.flatten()
		{
			Ok(codes) => codes,
			Err(err) => {
				error!("Failed to get codes: {err:#?}");
				continue;
			}
		};
		all_codes.extend(codes);
	}
	all_codes.sort_by_key(|e| e.code);
	let json = serde_json::to_string_pretty(&all_codes).unwrap();
	println!("{json}");
}
