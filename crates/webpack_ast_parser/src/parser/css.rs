use explorer_types::{ModuleId, serde_span::deserialize_span};
use oxc::{
	ast::ast::{ObjectProperty, ObjectPropertyKind},
	span::{GetSpan as _, Span},
};
use parser_diag::{PResult, err, err_ns};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, ToSmolStr};

use crate::{WebpackAstParser, parser::value_tracking::ConstantValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssModule {
	pub class_names: Vec<ClassName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassName {
	pub module: ModuleId,
	pub exported_as: SmolStr,
	#[serde(deserialize_with = "deserialize_span")]
	/// The span of the value def
	pub span: Span,
	/// The class name(s) from this export
	/// may be multiple space delimited if it uses composition
	pub value: String,
}

impl<'ast> WebpackAstParser<'ast> {
	#[expect(clippy::future_not_send)]
	pub(super) async fn impl_as_css_module(
		&self,
	) -> PResult<CssModule> {
		let raw_map = self.get_export_map_raw();
		let cn_obj = raw_map
			.node
			.ok_or_else(|| err_ns("raw export map does not have node set"))?;
		let cn_obj = cn_obj
			.as_object_expression()
			.ok_or_else(|| {
				err(&cn_obj, "raw_map::node is not an object literal")
			})?;
		if cn_obj.properties.is_empty() {
			return Err(err(cn_obj, "empty object literal is not a classnames module"));
		}
		let mut class_names = Vec::with_capacity(cn_obj.properties.len());
		for prop in &cn_obj.properties {
			let ObjectPropertyKind::ObjectProperty(prop) = prop else {
				return Err(err(
					prop,
					"css modules don't have spread properties",
				));
			};
			let prop = &**prop;
			let val = self
				.parse_prop(prop)
				.await
				.map_err(|e| {
					err(prop, "Failed to parse css class name module property")
						.s(e)
				})?;
			class_names.push(val);
		}
		Ok(CssModule { class_names })
	}

	#[expect(clippy::future_not_send)]
	async fn parse_prop(
		&self,
		prop: &'ast ObjectProperty<'ast>,
	) -> PResult<ClassName> {
		let exported_as = prop
			.key
			.static_name()
			.ok_or_else(|| {
				err(&prop.key, "property does not have a static name")
			})?
			.to_smolstr();
		let val = self
			.resolve_constant_value(&prop.value)
			.await
			.map_err(|e| {
				err(&prop.value, "Failed to resolve class name value").s(e)
			})?;
		let ConstantValue::String(value) = val.value else {
			return Err(err(&val, "resolved value is not a string"));
		};
		Ok(ClassName {
			module: self.get_module_id()?.id,
			exported_as,
			span: prop.value.span(),
			value,
		})
	}
}
