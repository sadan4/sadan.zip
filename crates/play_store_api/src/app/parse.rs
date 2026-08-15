use std::collections::HashMap;

use anyhow::{Context as _, Result, anyhow};
use ast_parser::{
	exts::{ExpressionExt, ObjectExpressionExt, StatementExt},
	parse_no_sema,
};
use html_parser::{Dom, Node};
use oxc::{
	ast::ast::{Argument, Expression, Program},
	span::{GetSpan as _, SourceType, Span},
};
use oxc_allocator::Allocator;
use smol_str::{SmolStr, ToSmolStr};
use tracing::debug;

use crate::{
	diag::{self, LocalSource, PResult},
	parse_json::ast2json,
};

#[derive(Debug, Clone, Copy)]
struct ScriptElement<'dom> {
	txt: Option<&'dom str>,
	src: Option<&'dom str>,
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
				// Not javascript; skip
				} else if e
					.attributes
					.get("type")
					.and_then(|t| t.as_ref())
					.is_some_and(|t| t == "application/ld+json")
				{
					return;
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

fn parse_script<'a>(
	alloc: &'a Allocator,
	script: &'a str,
) -> Result<Program<'a>> {
	parse_no_sema(alloc, script, SourceType::script())
		.map_err(|e| {
			let printer = LocalSource {
				name: "file.js",
				source: script,
				inner: e,
			};
			anyhow!("{printer:?}")
		})
		.context("Failed to parse script")
}

struct SingleScriptData {
	key: SmolStr,
	value: serde_json::Value,
}

fn extract_script_data(p: &Program<'_>) -> PResult<SingleScriptData> {
	if p.body.len() != 1 {
		return Err(diag::err(
			&p.span,
			"Expected body to be exactly one statement",
		));
	}
	let stmt = &p.body[0];
	let expr = &stmt
		.as_expression_statement()
		.ok_or_else(|| diag::err(stmt, "Expected an ExpressionStatement"))?
		.expression;
	let expr = expr
		.as_call_expression()
		.ok_or_else(|| diag::err(expr, "Expected a CallExpression"))?;
	let callee = match &expr.callee {
		Expression::Identifier(i) => i.as_ref(),
		other => {
			return Err(diag::err(
				other,
				"Expected callee to be a StaticIdentifier",
			));
		}
	};
	if callee.name != "AF_initDataCallback" {
		return Err(diag::err(
			callee,
			"Expected callee to be `AF_initDataCallback`",
		));
	}
	let arg_obj = match expr.arguments.as_slice() {
		[] | [_, _, ..] => {
			let s = expr.callee.span().end;
			let e = expr.span.end;
			return Err(diag::err(
				&Span::new(s, e),
				"Expected exactly one argument to `AF_initDataCallback`",
			));
		}
		[Argument::ObjectExpression(obj)] => obj.as_ref(),
		[other] => {
			return Err(diag::err(
				other,
				"expected argument to `AF_initDataCallback` to be an ObjectExpression",
			));
		}
	};
	let key_prop = arg_obj
		.get_property("key")
		.ok_or_else(|| {
			diag::err(arg_obj, "Expected object to have a `key` prop")
		})?;
	let key = key_prop
		.value
		.as_string_literal_like()
		.ok_or_else(|| {
			diag::err(key_prop, "expected `key` prop to be a StringLiteralish")
		})?
		.to_smolstr();
	let data_val = &arg_obj
		.get_property("data")
		.ok_or_else(|| {
			diag::err(arg_obj, "Expected object to have a `data` prop")
		})?
		.value;
	let value = ast2json(data_val)?;
	Ok(SingleScriptData { key, value })
}

fn collect_script_data(
	scripts: &[Program],
) -> HashMap<SmolStr, serde_json::Value> {
	scripts
		.iter()
		.filter_map(|p| match extract_script_data(p) {
			Ok(SingleScriptData { key, value }) => Some((key, value)),
			Err(e) => {
				debug!(
					"Failed to extract script data: {:?}",
					e.with_local_source(p.source_text, "script.js")
				);
				None
			}
		})
		.collect()
}

#[derive(Debug)]
pub struct ParsedHtml {
	pub(crate) script_data: HashMap<SmolStr, serde_json::Value>,
}

pub(super) fn parse(html: &str) -> Result<ParsedHtml> {
	let dom = Dom::parse(html).context("Failed to parse html")?;
	let alloc = Allocator::new();
	let scripts = collect_script_elements(&dom)
		.into_iter()
		.filter_map(|s| Some(parse_script(&alloc, s.txt?)))
		.collect::<Result<Vec<_>>>()
		.context("Failed to parse scripts")?;
	let script_data = collect_script_data(&scripts);
	Ok(ParsedHtml { script_data })
}
