use std::collections::HashMap;

use ast_parser::exts::{ExpressionExt, NumericLiteralExt as _};
use explorer_types::ModuleId;
use oxc::ast::ast::{ObjectExpression, ObjectPropertyKind};

use crate::{Sealed, types::ModuleEntry};

pub(crate) trait WebpackChunkParserImpl<'ast>: Sealed {
	/// the object with each module defined, should conform to `Record<PropertyKey, (e, t, n) => void)>`
	fn get_module_object(&self) -> Option<&'ast ObjectExpression<'ast>>;
	fn get_source_text(&self) -> &'ast str;
	fn try_parse_chunk_entry(
		&self,
		entry: &'ast ObjectPropertyKind<'ast>,
	) -> Option<ModuleEntry> {
		let entry = entry.as_property()?;
		let key = entry
			.key
			.as_numeric_literal()?
			.as_u32()
			.expect("non-integer module ids are not supported yet");
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
			let func = entry.value.as_function_expression()?;
			// function(...) {...} of `{foo: function(...) {...}}`
			let body = &self.get_source_text()[func.span];
			body.to_string()
			// format!("{body}")
		};
		Some(ModuleEntry(key.into(), src))
	}
}

pub trait WebpackChunkParser<'ast> {
	fn collect_defined_module_into(
		&self,
		target: &mut impl Extend<(ModuleId, String)>,
	) -> Option<()>;
	fn get_defined_modules(&self) -> Option<HashMap<ModuleId, String>> {
		let mut ret = HashMap::new();
		self.collect_defined_module_into(&mut ret)?;
		Some(ret)
	}
}

impl<'ast, T: WebpackChunkParserImpl<'ast>> WebpackChunkParser<'ast> for T {
	fn collect_defined_module_into(
		&self,
		target: &mut impl Extend<(ModuleId, String)>,
	) -> Option<()> {
		let other = self
			.get_module_object()?
			.properties
			.iter()
			.filter_map(|entry| self.try_parse_chunk_entry(entry))
			.map(Into::into);
		target.extend(other);
		Some(())
	}
}
