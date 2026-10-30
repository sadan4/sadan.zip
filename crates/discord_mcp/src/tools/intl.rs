//! Intl message lookup and search

use std::{fmt::Write as _, sync::Arc};

use anyhow::{Context as _, Result};
use discord_scraper::ScrapeProgress;
use explorer_types::intl::{IntlMessage, IntlMessages};
use serde_json::{Map, Value};
use smol_str::SmolStr;
use webpack_ast_parser::intl::{
	ast::hydrate::hydrate_ast,
	render_message,
	resolve_unhashed_key,
};

use crate::{store::LoadedBundle, text::truncate};

/// Renders an intl message to plain text, `{placeholders}` left as is
pub fn render(msg: &IntlMessage) -> Result<String> {
	let ast = match &msg.value {
		Value::Array(nodes) => nodes.clone(),
		// a message that is a single literal isn't wrapped in an array
		lit @ Value::String(_) => vec![lit.clone()],
		other => return Ok(other.to_string()),
	};
	let nodes = hydrate_ast(ast).context("Failed to hydrate intl message")?;
	let mut out = String::new();
	render_message(&mut out, &nodes, &Map::new())?;
	Ok(out)
}

pub fn hash_key(name: &str) -> SmolStr {
	intl_hash::hash_message_key(name)
		.iter()
		.collect::<String>()
		.into()
}

/// Accepts a hashed key, an unhashed key name, or Vencord's `#{intl::NAME}`
fn resolve_key(intl: &IntlMessages, key: &str) -> (SmolStr, Option<SmolStr>) {
	let key = key.trim();
	let key = key
		.strip_prefix("#{intl::")
		.and_then(|k| k.strip_suffix('}'))
		.unwrap_or(key);
	if let Some(raw) = key.strip_suffix("::raw") {
		return (raw.into(), resolve_unhashed_key(raw));
	}
	if intl.messages.contains_key(key) {
		return (key.into(), resolve_unhashed_key(key));
	}
	(hash_key(key), Some(key.into()))
}

fn describe(
	out: &mut String,
	hashed: &str,
	name: Option<&str>,
	msg: &IntlMessage,
) {
	if let Some(name) = name {
		writeln!(out, "{name} (hash {hashed}), module {}", msg.module).unwrap();
		writeln!(out, "  vencord: #{{intl::{name}}}").unwrap();
	} else {
		writeln!(out, "hash {hashed} (name unknown), module {}", msg.module)
			.unwrap();
		writeln!(out, "  vencord: #{{intl::{hashed}::raw}}").unwrap();
	}
	match render(msg) {
		Ok(text) => writeln!(out, "  text: {text}").unwrap(),
		Err(e) => writeln!(out, "  text: (failed to render: {e})").unwrap(),
	}
}

pub async fn intl_lookup(
	b: Arc<LoadedBundle>,
	key: &str,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	let intl = b.intl(progress).await?;
	let (hashed, name) = resolve_key(&intl, key);
	let Some(msg) = intl.messages.get(&hashed) else {
		return Ok(format!(
			"No intl message {key:?} (hash {hashed}) in build {}. Use \
			 intl_search to search by text.",
			b.id
		));
	};
	let mut out = String::new();
	describe(&mut out, &hashed, name.as_deref(), msg);
	Ok(out)
}

pub async fn intl_search(
	b: Arc<LoadedBundle>,
	query: &str,
	limit: usize,
	progress: Arc<dyn ScrapeProgress>,
) -> Result<String> {
	let intl = b.intl(progress).await?;
	let needle = query.trim().to_lowercase();
	let mut hits = intl
		.messages
		.iter()
		.filter_map(|(hashed, msg)| {
			let name = resolve_unhashed_key(hashed);
			let name_hit = name
				.as_ref()
				.is_some_and(|n| n.to_lowercase().contains(&needle));
			let text = render(msg).ok()?;
			let text_hit = text.to_lowercase().contains(&needle);
			(name_hit || text_hit).then_some((hashed, name, msg, text.len()))
		})
		.collect::<Vec<_>>();
	// shortest messages first, they're most likely the exact string wanted
	hits.sort_unstable_by(|a, b| a.3.cmp(&b.3).then_with(|| a.0.cmp(b.0)));
	if hits.is_empty() {
		return Ok(format!("No intl message matches {query:?}"));
	}
	let mut out = format!("{} message(s) match {query:?}\n\n", hits.len());
	for (hashed, name, msg, _) in hits.iter().take(limit) {
		describe(&mut out, hashed, name.as_deref(), msg);
	}
	if hits.len() > limit {
		write!(out, "\n[{} more not shown]", hits.len() - limit).unwrap();
	}
	Ok(truncate(out, "Use a more specific query or lower limit"))
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use explorer_types::ModuleId;

	use super::*;

	fn messages() -> IntlMessages {
		let mut messages = HashMap::new();
		messages.insert(
			hash_key("HELLO_WORLD"),
			IntlMessage {
				module: ModuleId(5),
				value: Value::String("Hello, world!".into()),
			},
		);
		IntlMessages {
			modules: vec![ModuleId(5)],
			messages,
		}
	}

	#[test]
	fn resolves_every_key_form() {
		let intl = messages();
		let hashed = hash_key("HELLO_WORLD");
		for key in ["HELLO_WORLD", "#{intl::HELLO_WORLD}", hashed.as_str()] {
			assert_eq!(resolve_key(&intl, key).0, hashed, "{key}");
		}
	}

	#[test]
	fn renders_literal() {
		let intl = messages();
		let msg = intl.messages.values().next().unwrap();
		assert_eq!(render(msg).unwrap(), "Hello, world!");
	}
}
