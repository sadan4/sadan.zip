#![allow(clippy::unreadable_literal, clippy::too_many_lines)]

use super::*;
use ast_parser::span_line_and_column;
use insta::assert_debug_snapshot;
use itertools::Itertools;
use macros::test;
use oxc::{ast::ast::Str, span::Span};
use std::fmt::{self, Debug};

#[macro_export]
macro_rules! parse {
	($source:literal) => {{
		super let alloc = ::oxc::allocator::Allocator::new();
		let source = include_str!($source);
		$crate::WebpackAstParser::try_new(&alloc, source).unwrap()
	}};
}

mod concatenated_modules;
mod direct_module_definition;
mod experiments;
mod export_parsing;
mod find_gen;
mod import_parsing;
mod intl_keys;
mod is_write_once;
mod key_modules;
mod module_id;
mod outgoing_deps;
mod remaining_access_chain;

#[derive(Copy, Clone)]
struct SpanDumper<'a>(pub Span, pub &'a str);

impl Debug for SpanDumper<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ((l1, c1), (l2, c2)) = span_line_and_column(self.1, self.0);
		let text = self.1[self.0].escape_debug();
		write!(f, r#""[{l1}:{c1}->{l2}:{c2}) {text}""#)
	}
}

struct ExportMapDumper<'a>(pub &'a RangeExportMap, pub &'a str);

impl ExportMapDumper<'_> {
	fn handle_value(
		&self,
		f: &mut fmt::Formatter<'_>,
		v: &RangeExportMapValue,
	) -> Result<(), fmt::Error> {
		match v {
			ExportValue::Range(range) => {
				let do_dbg_list = fmt::from_fn(|f| {
					let mut dbg_list = f.debug_list();
					for &span in range.iter() {
						let ((l1, c1), (l2, c2)) =
							span_line_and_column(self.1, span);
						let text = self.1[span].escape_debug();
						dbg_list
							.entry(&format!("[{l1}:{c1}->{l2}:{c2}) {text}"));
					}
					dbg_list.finish()
				});
				if let Some(hover) = &range.1 {
					f.debug_tuple(hover.as_str())
						.field(&do_dbg_list)
						.finish()
				} else {
					do_dbg_list.fmt(f)
				}
			}
			ExportValue::Map(m) => {
				let dumper = ExportMapDumper(m, self.1);
				f.debug_tuple(
					m.hover
						.as_ref()
						.map_or("ExportMap", SmolStr::as_str),
				)
				.field(&dumper)
				.finish()
			}
		}
	}
}

impl Debug for ExportMapDumper<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut dbg_map = f.debug_map();
		for (k, v) in self
			.0
			.exports
			.iter()
			.sorted_by(|a, b| a.0.cmp(b.0))
		{
			dbg_map.entry(&k, &fmt::from_fn(|f| self.handle_value(f, v)));
		}
		if let Some(v) = &self.0.cjs_default {
			let v = v.as_ref();
			dbg_map.entry(
				&"SYM_CJS_DEFAULT",
				&fmt::from_fn(|f| self.handle_value(f, v)),
			);
		}

		dbg_map.finish()
	}
}

impl<'ast> WebpackAstParser<'ast> {
	fn t_sym_info<'a>(&'a self, sym_id: SymbolId) -> (Str<'a>, Span)
	where
		'ast: 'a,
	{
		let name = self
			.sema
			.scoping()
			.symbol_ident(sym_id)
			.as_arena_str();
		let span = self
			.sema
			.scoping()
			.symbol_declaration(sym_id);
		let node = self.n(span);
		(name, node.span())
	}
	fn dbg_export_map(&self) -> ExportMapDumper<'_> {
		ExportMapDumper(self.get_export_map(), self.source)
	}
	#[expect(clippy::type_complexity)]
	fn dbg_outgoing_deps(
		&self,
	) -> (
		Vec<(ModuleId, SpanDumper<'_>)>,
		Vec<(ModuleId, SpanDumper<'_>)>,
	) {
		let deps = self
			.get_modules_that_this_module_requires()
			.cloned()
			.unwrap_or_default();
		let map = |v: Vec<SpannedId>| {
			v.into_iter()
				.map(|s| (s.id, SpanDumper(s.span, self.source)))
				.sorted_by_key(|(id, _)| id.0)
				.collect::<Vec<_>>()
		};
		(map(deps.sync), map(deps.lazy))
	}
	fn dbg_intl_keys(&self) -> Vec<(SpanDumper<'_>, SmolStr, Option<SmolStr>)> {
		self.get_intl_keys()
			.into_iter()
			.map(|(span, key)| {
				(SpanDumper(span, self.source), key.hashed, key.unhashed)
			})
			.collect()
	}
	fn dbg_uses_of_import<'a>(
		&'a self,
		module_id: ModuleId,
		export_names: &[ExportMapKey],
	) -> Vec<SpanDumper<'a>> {
		self.get_uses_of_import(module_id, export_names)
			.into_iter()
			.sorted()
			.map(|span| SpanDumper(span, self.source))
			.collect()
	}
}

#[test]
fn constructs() {
	let alloc = Allocator::new();
	let source = include_str!("test_data/wp/module.js");
	_ = WebpackAstParser::try_new(&alloc, source).unwrap();
}

#[test]
fn finds_wreq() {
	let p = parse!("test_data/wp/module.js");
	let wreq = p.wreq().unwrap();
	let info = p.t_sym_info(wreq);
	assert_debug_snapshot!(info, @r#"
	(
	    "n",
	    Span {
	        start: 56,
	        end: 57,
	    },
	)
	"#);
}

#[test]
fn doesnt_find_wreq_in_module_that_doesnt_use_it() {
	let p = parse!("test_data/wp/bad/noWreq.js");
	assert!(p.wreq().is_err());
}

#[test]
fn finds_imported_var() {
	let p = parse!("test_data/wp/module.js");
	let info = p
		.get_imported_var(200651.into())
		.unwrap();
	let info = p.t_sym_info(info);
	assert_debug_snapshot!(info, @r#"
	(
	    "r",
	    Span {
	        start: 181,
	        end: 194,
	    },
	)
	"#);
}

#[test]
fn doesnt_find_side_effect_import() {
	let p = parse!("test_data/wp/module.js");
	let info = p.get_imported_var(411104.into());
	assert_eq!(info, None);
}
