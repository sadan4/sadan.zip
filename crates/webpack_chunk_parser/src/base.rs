use std::collections::HashMap;

use ast_parser::exts::{ExpressionExt, NumericLiteralExt as _};
use explorer_types::ModuleId;
use oxc::ast::ast::{ObjectExpression, ObjectPropertyKind};
use parser_diag::{LocalSource, PResult, err};
use tracing::warn;

use crate::{Sealed, types::ModuleEntry};

pub(crate) trait WebpackChunkParserImpl<'ast>: Sealed {
	/// the object with each module defined, should conform to `Record<PropertyKey, (e, t, n) => void)>`
	fn get_module_object(&self) -> PResult<&'ast ObjectExpression<'ast>>;
	fn get_source_text(&self) -> &'ast str;
	fn try_parse_chunk_entry(
		&self,
		entry: &'ast ObjectPropertyKind<'ast>,
	) -> PResult<ModuleEntry> {
		let entry = entry
			.as_property()
			.ok_or_else(|| err(entry, "entry is not an object property"))?;
		let key = entry
			.key
			.as_numeric_literal()
			.ok_or_else(|| err(&entry.key, "key is not a numeric literal"))?
			.as_u32()
			.ok_or_else(|| err(&entry.key, "key is not a u32"))?;
		let src = if entry.method {
			// entry.method is true so we must be a function
			let func = entry
				.value
				.as_function_expression()
				.unwrap();
			// (...) {...} of `{foo(...) {...}}`
			let body = &self.get_source_text()[func.span];
			format!("function{body}")
		} else {
			let func = entry
				.value
				.as_function_expression()
				.ok_or_else(|| {
					err(&entry.value, "value is not a function expression")
				})?;
			// function(...) {...} of `{foo: function(...) {...}}`
			let body = &self.get_source_text()[func.span];
			body.to_string()
			// format!("{body}")
		};
		Ok(ModuleEntry(key.into(), src))
	}
}

pub trait WebpackChunkParser<'ast> {
	fn collect_defined_modules(
		&self,
	) -> PResult<impl Iterator<Item = (ModuleId, String)>>;
	fn get_defined_modules(&self) -> PResult<HashMap<ModuleId, String>> {
		let ret = HashMap::from_iter(self.collect_defined_modules()?);
		Ok(ret)
	}
}

impl<'ast, T: WebpackChunkParserImpl<'ast>> WebpackChunkParser<'ast> for T {
	fn collect_defined_modules(
		&self,
	) -> PResult<impl Iterator<Item = (ModuleId, String)>> {
		let other = self
			.get_module_object()?
			.properties
			.iter()
			.filter_map(|entry| match self.try_parse_chunk_entry(entry) {
				Ok(s) => Some(s),
				Err(e) => {
					let e = LocalSource {
						inner: e.into(),
						source: self.get_source_text(),
						name: "file.js"
					};
					warn!("Failed to parse chunk entry: {e:?}");
					None
				},
			})
			.map(Into::into);
		Ok(other)
	}
}
