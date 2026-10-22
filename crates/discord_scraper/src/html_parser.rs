use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use const_format::formatc;
use html_parser::{Dom, Node};
use regress::Regex;
use webpack_chunk_parser::ChunkHashEntry;

pub struct ParsedHtml {
	pub global_env_text: String,
	pub web_js_url: String,
	pub extra_chunks: Vec<ChunkHashEntry>,
	pub extra_css_chunks: Vec<ChunkHashEntry>,
}

#[derive(Debug)]
struct ScriptElement<'dom> {
	txt: Option<&'dom str>,
	src: Option<&'dom str>,
}

struct CssLink<'dom> {
	href: &'dom str,
}

const GLOBAL_ENV_NEEDLE: &str = "window.GLOBAL_ENV =";
const SRC_PREFIX: &str = "/assets/";
const WEB_JS_PREFIX: &str = "web.";
const WEB_JS_FULL_PREFIX: &str = formatc!("{SRC_PREFIX}{WEB_JS_PREFIX}");
static CHUNK_SCRIPT_HREF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r"/assets/(([\w.-]+?)\.[a-fA-F\d]{16})\.js")
		.expect("invalid regex")
});
static CHUNK_CSS_HREF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r"/assets/(([\w.-]+?)\.[a-fA-F\d]{16})\.css")
		.expect("invalid regex")
});

pub fn parse_html(html: &str) -> Result<ParsedHtml> {
	let dom = Dom::parse(html)?;
	if !dom.errors.is_empty() {
		bail!("failed to parse html: {:?}", dom.errors);
	}
	let scripts = collect_script_elements(&dom);
	let stylesheets = collect_css_link_elements(&dom);
	let global_env_text = scripts
		.iter()
		.find_map(|s| {
			s.txt.and_then(|txt| {
				txt.contains(GLOBAL_ENV_NEEDLE)
					.then_some(txt)
			})
		})
		.map(str::to_owned)
		.context("Could not find env script element")?;
	let web_js_url = scripts
		.iter()
		.find_map(|s| {
			s.src.and_then(|src| {
				src.starts_with(WEB_JS_FULL_PREFIX)
					.then(|| &src[SRC_PREFIX.len()..])
			})
		})
		.map(str::to_owned)
		.context("Could not find web.js entrypoint")?;
	let extra_chunks = scripts
		.into_iter()
		.filter_map(|s| {
			let src = s.src?;
			if src.starts_with(WEB_JS_FULL_PREFIX) {
				return None;
			}
			CHUNK_SCRIPT_HREF_REGEX
				.find(src)
				.map(|m| ChunkHashEntry {
					chunk_id: src[m.group(2).unwrap()].into(),
					hash: src[m.group(1).unwrap()].into(),
				})
		})
		.collect();
	let extra_css_chunks = stylesheets
		.into_iter()
		.filter_map(|CssLink { href }| {
			CHUNK_CSS_HREF_REGEX
				.find(href)
				.map(|m| ChunkHashEntry {
					chunk_id: href[m.group(2).unwrap()].into(),
					hash: href[m.group(1).unwrap()].into(),
				})
		})
		.collect();
	Ok(ParsedHtml {
		global_env_text,
		web_js_url,
		extra_chunks,
		extra_css_chunks,
	})
}

fn collect_css_link_elements(dom: &Dom) -> Vec<CssLink<'_>> {
	const TAG: &str = "link";
	fn do_<'vec, 'dom: 'vec>(
		node: &'dom Node,
		elts: &'vec mut Vec<CssLink<'dom>>,
	) {
		let Node::Element(e) = node else {
			return;
		};

		if e.name == TAG {
			if e.attributes
				.get("rel")
				.and_then(Option::as_ref)
				.is_none_or(|rel| rel != "stylesheet")
			{
				return;
			}

			let Some(href) = e
				.attributes
				.get("href")
				.and_then(Option::as_deref)
			else {
				return;
			};

			elts.push(CssLink { href });
		} else {
			for c in &e.children {
				do_(c, elts);
			}
		}
	}
	let mut ret = Vec::new();
	for c in &dom.children {
		do_(c, &mut ret);
	}
	ret
}

fn collect_script_elements(dom: &Dom) -> Vec<ScriptElement<'_>> {
	const SCRIPT_TAG: &str = "script";
	const SCRIPT_SRC_ATTR: &str = "src";
	fn do_<'vec, 'dom: 'vec>(
		node: &'dom Node,
		scripts: &'vec mut Vec<ScriptElement<'dom>>,
	) {
		if let Node::Element(e) = node {
			if e.name == SCRIPT_TAG {
				let txt = if e.children.is_empty() {
					None
				} else if e.children.len() > 1 {
					panic!(
						"script elements should not have more than one child"
					);
				} else if let Node::Text(txt) = &e.children[0] {
					Some(txt.as_str())
				} else {
					panic!("script element child should be text");
				};
				let src = e
					.attributes
					.get(SCRIPT_SRC_ATTR)
					.and_then(Option::as_ref)
					.map(String::as_str);
				scripts.push(ScriptElement { txt, src });
			} else {
				// script tags can not have child nodes in html
				for child in &e.children {
					do_(child, scripts);
				}
			}
		}
	}
	let mut scripts = Vec::new();
	for node in &dom.children {
		do_(node, &mut scripts);
	}
	scripts
}
