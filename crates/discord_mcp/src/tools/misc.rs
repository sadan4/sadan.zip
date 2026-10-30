//! Experiments, css classes and REST endpoints

use std::{fmt::Write as _, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use discord_scraper::ScrapeProgress;
use explorer_types::ModuleId;
use memchr::memmem;
use webpack_ast_parser::{
	css::ClassName,
	endpoints::{Endpoint, EndpointParam, Segment},
};

use crate::{store::LoadedBundle, text::truncate};

pub async fn list_experiments(
	b: Arc<LoadedBundle>,
	query: &str,
	limit: usize,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	let experiments = b.experiments(progress).await?;
	let needle = query.trim().to_lowercase();
	let hits = experiments
		.iter()
		.filter_map(|e| {
			let json = serde_json::to_string(&e.obj).ok()?;
			json.to_lowercase()
				.contains(&needle)
				.then_some((e, json))
		})
		.collect::<Vec<_>>();
	if hits.is_empty() {
		return Ok(format!("No experiment matches {query:?}"));
	}
	let mut out = format!("{} experiment(s) match\n", hits.len());
	for (e, json) in hits.iter().take(limit) {
		let json = &json[..json.floor_char_boundary(400)];
		writeln!(out, "\nmodule {}: {json}", e.loc.id).unwrap();
	}
	if hits.len() > limit {
		write!(out, "\n[{} more not shown]", hits.len() - limit).unwrap();
	}
	Ok(truncate(out, "Use a more specific query or lower limit"))
}

pub async fn all_css_classes(
	b: &Arc<LoadedBundle>,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<Arc<Vec<ClassName>>> {
	b.caches
		.css
		.get_or_try_init(|| async {
			let parsed = b.parsed(progress).await?;
			let mut classes = Vec::new();
			for (_, parser) in parsed.parsers() {
				if let Ok(css) = parser.parser().as_css_module().await {
					classes.extend(css.class_names);
				}
			}
			classes.sort_unstable_by(|a, b| {
				(a.module, &a.exported_as).cmp(&(b.module, &b.exported_as))
			});
			Ok(Arc::new(classes))
		})
		.await
		.cloned()
}

pub async fn css_classes(
	b: Arc<LoadedBundle>,
	query: &str,
	limit: usize,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	let classes = all_css_classes(&b, progress).await?;
	let needle = query.trim();
	let hits = classes
		.iter()
		.filter(|c| c.exported_as.contains(needle) || c.value.contains(needle))
		.collect::<Vec<_>>();
	if hits.is_empty() {
		return Ok(format!("No css class matches {query:?}"));
	}
	let mut out = format!(
		"{} css class(es) match. Format: module: exported name -> class \
		 name(s). css_search with target=selector shows their rules.\n\n",
		hits.len()
	);
	for c in hits.iter().take(limit) {
		writeln!(out, "{}: {} -> {}", c.module, c.exported_as, c.value)
			.unwrap();
	}
	if hits.len() > limit {
		write!(out, "\n[{} more not shown]", hits.len() - limit).unwrap();
	}
	Ok(truncate(out, "Use a more specific query or lower limit"))
}

pub struct EndpointsModule {
	pub module: ModuleId,
	pub endpoints: Vec<Endpoint>,
}

/// Keys that only the endpoints module has all of
const ENDPOINT_MARKERS: &[&str] = &[
	"GUILD_CHANNELS",
	"GUILD_MEMBER_NICK",
	"MESSAGE_ACK",
	"USERS",
	"ME",
];

async fn endpoints_of(b: &Arc<LoadedBundle>) -> Result<Arc<EndpointsModule>> {
	b.caches
		.endpoints
		.get_or_try_init(|| async {
			let finders = ENDPOINT_MARKERS
				.iter()
				.map(memmem::Finder::new)
				.collect::<Vec<_>>();
			for id in b.sorted_ids() {
				let src = b.raw(id)?;
				if !finders
					.iter()
					.all(|f| f.find(src.as_bytes()).is_some())
				{
					continue;
				}
				let parser = b.standalone_parser(id)?;
				if let Some(endpoints) = parser.parser().as_endpoints_module() {
					let endpoints = endpoints.map_err(|e| anyhow!("{e}"))?;
					return Ok(Arc::new(EndpointsModule {
						module: id,
						endpoints,
					}));
				}
			}
			Err(anyhow!("Could not find the endpoints module"))
		})
		.await
		.cloned()
}

fn render_segments(out: &mut String, segments: &[Segment]) {
	for segment in segments {
		match segment {
			Segment::Literal { value } => out.push_str(value),
			Segment::Param { index } => write!(out, "${{p{index}}}").unwrap(),
			Segment::EscapedParam { index } => {
				write!(out, "${{escape(p{index})}}").unwrap();
			}
			Segment::Global { path } => write!(out, "${{{path}}}").unwrap(),
			Segment::Conditional {
				param,
				consequent,
				alternate,
			} => {
				write!(out, "${{p{param} != null ? `").unwrap();
				render_segments(out, consequent);
				out.push_str("` : `");
				render_segments(out, alternate);
				out.push_str("`}");
			}
		}
	}
}

pub fn render_endpoint(e: &Endpoint) -> String {
	let mut out = e.name.clone();
	if e.is_function {
		let params = e
			.params
			.iter()
			.enumerate()
			.map(|(i, p)| match p {
				EndpointParam::Required => format!("p{i}"),
				EndpointParam::Optional { default } => {
					format!("p{i} = {default}")
				}
			})
			.collect::<Vec<_>>();
		write!(out, "({})", params.join(", ")).unwrap();
	}
	out.push_str(": `");
	render_segments(&mut out, &e.value);
	out.push('`');
	out
}

pub async fn endpoints(
	b: Arc<LoadedBundle>,
	query: &str,
	limit: usize,
) -> Result<String> {
	let module = endpoints_of(&b)
		.await
		.context("Failed to read endpoints")?;
	let needle = query.trim().to_lowercase();
	let hits = module
		.endpoints
		.iter()
		.map(render_endpoint)
		.filter(|e| e.to_lowercase().contains(&needle))
		.collect::<Vec<_>>();
	let mut out = format!(
		"{} endpoint(s) match in module {}. Access as Endpoints.NAME\n\n",
		hits.len(),
		module.module
	);
	for e in hits.iter().take(limit) {
		writeln!(out, "{e}").unwrap();
	}
	if hits.len() > limit {
		write!(out, "\n[{} more not shown]", hits.len() - limit).unwrap();
	}
	Ok(truncate(out, "Use a more specific query or lower limit"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn renders_endpoint_template() {
		let e = Endpoint {
			name: String::from("GUILD_CHANNELS"),
			value: vec![
				Segment::Literal {
					value: String::from("/guilds/"),
				},
				Segment::Param { index: 0 },
				Segment::Literal {
					value: String::from("/channels"),
				},
			],
			params: vec![EndpointParam::Required],
			is_function: true,
		};
		assert_eq!(
			render_endpoint(&e),
			"GUILD_CHANNELS(p0): `/guilds/${p0}/channels`"
		);
	}
}
