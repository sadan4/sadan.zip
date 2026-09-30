use crate::parser::{
	WebpackAstParser,
	export_map::ExportMapKey,
	util::{f64_to_i32, get_inner_func_body},
};
use ast_parser::{
	AstParser,
	exts::{ExpressionExt, ObjectExpressionExt, StatementExt},
};
use explorer_types::{
	SpannedId,
	experiments::{
		self,
		ApexExperiment,
		Experiment,
		ExperimentKind,
		ExperimentScope,
		NormalExperiment,
	},
};
use oxc::ast::ast::{Expression, ObjectExpression, ObjectProperty};
use parser_diag::{PResult, err};
use tracing::debug;

impl<'ast> WebpackAstParser<'ast> {
	// nothing outside of here uses this as of now
	pub(crate) fn find_exported_func_by_returned_obj_props(
		&self,
		props: &[&str],
	) -> Option<ExportMapKey> {
		// iterate over the export map keys
		// find a function that returns an object with the following properties
		// definition, useConfig, getConfig
		let map = self.get_export_map_raw();
		let it = map.exports.iter().filter_map(|(k, v)| {
			Some((k, *v.try_unwrap_range_ref().ok()?.last()?))
		});
		for (key, val) in it {
			try {
				debug!("val: {}", val.debug_name());
				let ident = val.as_binding_identifier()?;
				let func = self.p(ident.node_id()).as_function()?;
				debug!("is func");
				let body = get_inner_func_body(func);
				let mut ret = body
					.statements
					.last()?
					.as_return_statement()?
					.argument
					.as_ref()?
					.get_inner_expression();
				// `return foo(), { ... };`
				if let Expression::SequenceExpression(seq) = ret {
					ret = seq
						.expressions
						.last()?
						.get_inner_expression();
				}
				let ret_obj = ret.as_object_expression()?;
				debug!("ret is obj");
				for prop in props {
					ret_obj.get_property(prop)?;
				}
				return Some(key.clone().into());
			};
		}
		None
	}
	pub(crate) fn parse_apex_experiment(
		&self,
		obj: &'ast ObjectExpression<'ast>,
	) -> PResult<Experiment> {
		let name = self.parse_experiment_name(obj, "name")?;
		let default_config = if let Some(ObjectProperty { value, .. }) =
			obj.get_property("defaultConfig")
		{
			self.config_to_json(value, "`defaultConfig`")
		} else {
			serde_json::Value::Null
		};
		let label = if let Some(ObjectProperty { value, .. }) =
			obj.get_property("label")
		{
			let label = value
				.as_string_literal_like()
				.ok_or_else(|| {
					err(
						value,
						"`label` experiment property is not a string literal",
					)
				})?;
			Some(label.to_string())
		} else {
			None
		};
		let kind = Self::parse_experiment_scope(obj)?;
		let variations =
			if let Some(variations) = obj.get_property("variations") {
				let vars = variations
					.value
					.as_object_expression()
					.ok_or_else(|| {
						err(
							&variations.value,
							"`varitions` is not an object expression",
						)
					})?;
				let mut arr = Vec::with_capacity(vars.properties.len());
				for prop in &vars.properties {
					let Some(prop) = prop.as_property() else {
						return Err(err(prop, "Invalid variation property"));
					};
					let key = prop.key.static_name().ok_or_else(|| {
						err(
							&prop.key,
							"Variation property key is not an a static name",
						)
					})?;
					let val = &prop.value;
					let val = self.config_to_json(val, "variation value");
					arr.push(experiments::Variation {
						key: key.to_string(),
						config: val,
					});
				}
				arr
			} else {
				vec![]
			};
		Ok(Experiment {
			loc: SpannedId {
				id: self.get_module_id()?.id,
				span: obj.span,
			},
			obj: ExperimentKind::Apex(ApexExperiment {
				kind,
				name,
				default_config,
				label,
				variations,
			}),
		})
	}
	fn parse_experiment_scope(
		obj: &'ast ObjectExpression<'ast>,
	) -> PResult<ExperimentScope> {
		let kind = &obj
			.get_property("kind")
			.ok_or_else(|| err(obj, "Experiment does not have kind property"))?
			.value;
		let scope = kind
			.as_string_literal_like()
			.ok_or_else(|| {
				err(kind, "`kind` experiment property is not a string literal")
			})?;
		match scope.as_str() {
			"user" => Ok(ExperimentScope::User),
			"guild" => Ok(ExperimentScope::Guild),
			"installation" => Ok(ExperimentScope::Installation),
			_ => Err(err(
				kind,
				"`kind` experiment property is not `user` or `guild`",
			)),
		}
	}
	fn parse_experiment_treatment(
		&self,
		t: &'ast Expression<'ast>,
	) -> PResult<experiments::Treatment> {
		let t = t
			.as_object_expression()
			.ok_or_else(|| {
				err(t, "Experiment treatment is not an object literal")
			})?;
		let id_prop = t.get_property("id").ok_or_else(|| {
			err(t, "Experiment treatment does not have `id` property")
		})?;
		let id = id_prop
			.value
			.as_numeric_literal()
			.ok_or_else(|| {
				err(
					&id_prop.value,
					"`id` experiment treatment property is not a numeric literal",
				)
			})?;
		let id = f64_to_i32(id.value).ok_or_else(|| {
			err(&id_prop.value, "Failed to convert `id` to i32")
		})?;
		let label_prop = t.get_property("label").ok_or_else(|| {
			err(t, "Experiment treatment does not have `label` property")
		})?;
		let label = label_prop
			.value
			.as_string_literal_like()
			.ok_or_else(|| {
				err(
					&label_prop.value,
					"`label` experiment treatment property is not a string literal",
				)
			})?;
		let config_prop = t
			.get_property("config")
			.ok_or_else(|| {
				err(t, "Experiment treatment does not have `config` property")
			})?;
		let config = self.config_to_json(&config_prop.value, "`config`");
		Ok(experiments::Treatment {
			id,
			label: label.to_string(),
			config,
		})
	}

	/// Converts an experiment config to json.
	///
	/// returns [`null`](serde_json::Value::Null) if the config can't be converted to json
	fn config_to_json(
		&self,
		config: &'ast Expression<'ast>,
		what: &str,
	) -> serde_json::Value {
		self.expr_to_json(config)
			.unwrap_or_else(|e| {
				self.warn_diag(
					&format!("Failed to parse experiment {what}, using null"),
					e,
				);
				serde_json::Value::Null
			})
	}

	fn parse_experiment_name(
		&self,
		obj: &'ast ObjectExpression<'ast>,
		prop: &str,
	) -> PResult<String> {
		let name_prop = obj.get_property(prop).ok_or_else(|| {
			err(obj, format!("Experiment does not have `{prop}` property"))
		})?;
		let ret = match &name_prop.value {
			Expression::StringLiteral(lit) => lit.value.to_string(),
			Expression::TemplateLiteral(lit)
				if lit.is_no_substitution_template() =>
			{
				lit.quasis
					.first()
					.unwrap()
					.value
					.cooked
					.unwrap()
					.to_string()
			}
			Expression::Identifier(ident) => self
				.is_constant_string(&**ident)
				.ok_or_else(|| {
					err(
						&**ident,
						format!(
							"Could not resolve {prop} string from identifier"
						),
					)
				})?
				.to_string(),
			other => {
				return Err(err(
					other,
					format!("Could not resolve {prop} string"),
				));
			}
		};
		Ok(ret)
	}

	pub(crate) fn parse_normal_experiment(
		&self,
		obj: &'ast ObjectExpression<'ast>,
	) -> PResult<Experiment> {
		let kind = Self::parse_experiment_scope(obj)?;
		let id = self.parse_experiment_name(obj, "id")?;
		let label = &obj
			.get_property("label")
			.ok_or_else(|| err(obj, "Experiment does not have label property"))?
			.value;
		let label = label
			.as_string_literal_like()
			.ok_or_else(|| {
				err(
					label,
					"`label` experiment property is not a string literal",
				)
			})?;
		let default_config = if let Some(ObjectProperty { value, .. }) =
			obj.get_property("defaultConfig")
		{
			self.config_to_json(value, "`defaultConfig`")
		} else {
			serde_json::Value::Null
		};
		let treatments =
			if let Some(treatments) = obj.get_property("treatments") {
				let arr = treatments
					.value
					.as_array_expression()
					.ok_or_else(|| {
						err(
							&treatments.value,
							"`treatments` is not an array expression",
						)
					})?;
				let mut ret = Vec::with_capacity(arr.elements.len());
				for el in &arr.elements {
					let Some(el) = el.as_expression() else {
						return Err(err(el, "Invalid treatment element"));
					};
					ret.push(self.parse_experiment_treatment(el)?);
				}
				ret
			} else {
				vec![]
			};
		// TODO: parse the value once we know its shape
		let common_trigger_point = obj
			.get_property("commonTriggerPoint")
			.map(|_| ());
		Ok(Experiment {
			loc: SpannedId {
				id: self.get_module_id()?.id,
				span: obj.span,
			},
			obj: ExperimentKind::Normal(NormalExperiment {
				kind,
				id,
				label: label.to_string(),
				default_config,
				treatments,
				common_trigger_point,
			}),
		})
	}
}
