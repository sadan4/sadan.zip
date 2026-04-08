use crate::{
	Patch,
	ReplaceLike,
	Replacement,
	Replacer,
	TemplateEvaluator,
	pass::{
		EvalStringRawPass,
		FlattenTemplatePass,
		FoldBinaryExpressionsPass,
		InlineConstantsPass,
		InlineEnumsPass,
		PassManager,
	},
	patches::{
		RawMatchLike,
		RawPatch,
		RawReplace,
		RawReplacement,
		canonicalize_match_like,
		canonicalize_replace_for_regress,
	},
};
use anyhow::{Context, Result, bail};
use ast_parser::{
	AstParser,
	ESModuleParser,
	exts::{
		ArrayExpressionElementExt as _,
		BindingPatternExt as _,
		ExpressionExt as _,
		ImportDeclarationExt as _,
		ObjectExpressionExt as _,
	},
	parse_for_traverse,
};
use oxc::{
	allocator::{Allocator, HashMap as OxcHashMap, Vec as OxcVec},
	ast::{
		AstBuilder,
		AstKind,
		ast::{
			Argument,
			ArrayExpressionElement,
			ArrowFunctionExpression,
			Expression,
			ObjectExpression,
			Program,
			SpreadElement,
			StringLiteral,
		},
	},
	minifier::PropertyReadSideEffects,
	semantic::{Semantic, SymbolId},
	span::SourceType,
};
use oxc_ecmascript::{
	GlobalContext,
	constant_evaluation::{ConstantEvaluation, ConstantEvaluationCtx},
	side_effects::MayHaveSideEffectsContext,
};
use tracing::{debug, trace, warn};

pub struct VencordAstParser<'ast> {
	pub(crate) alloc: &'ast Allocator,
	pub(crate) ast_builder: AstBuilder<'ast>,
	pub(crate) prog: &'ast Program<'ast>,
	pub(crate) sema: Semantic<'ast>,
}

const DEFINE_PLUGIN_IMPORT_SOURCE: &str = "@utils/types";

// TODO: get webpack finds
impl<'ast> VencordAstParser<'ast> {
	pub fn try_new(alloc: &'ast Allocator, source: &'ast str) -> Result<Self> {
		let pass_data = parse_for_traverse(alloc, source, SourceType::tsx())?;

		let (prog, sema) = PassManager::new(alloc, pass_data)
			.run_pass(EvalStringRawPass)
			.run_pass(FoldBinaryExpressionsPass)
			.run_pass(InlineConstantsPass::default())
			.run_pass(InlineEnumsPass::default())
			.run_pass(FlattenTemplatePass)
			.run_pass(InlineConstantsPass::default()) // HACK: should not be needed
			.run_pass(FlattenTemplatePass)
			.finish();

		Ok(Self {
			alloc,
			ast_builder: AstBuilder::new(alloc),
			prog,
			sema,
		})
	}

	fn define_plugin<'a: 'ast>(
		&'a self,
	) -> Option<&'ast ObjectExpression<'ast>> {
		let define_plugin = self
			.find_import_by_name(DEFINE_PLUGIN_IMPORT_SOURCE)?
			.default_var()?;
		for var_use in self
			.sema
			.symbol_references(define_plugin)
		{
			let AstKind::CallExpression(call) = self.p(var_use.node_id())
			else {
				continue;
			};
			if call.arguments.len() != 1 {
				continue;
			}
			let Argument::ObjectExpression(obj) = &call.arguments[0] else {
				continue;
			};
			return Some(obj.as_ref());
		}
		None
	}

	fn plugin_name<'a: 'ast>(&'a self) -> Option<&'ast str> {
		Some(
			self.define_plugin()?
				.get_property("name")?
				.value
				.as_string_literal()?
				.value
				.as_str(),
		)
	}

	fn try_into_raw_match_like<'a: 'ast>(
		&'a self,
		value: &'ast Expression<'ast>,
	) -> Result<RawMatchLike<'ast>> {
		match value {
			Expression::RegExpLiteral(r) => Ok(RawMatchLike::Regex(r.as_ref())),
			Expression::StringLiteral(s) => {
				Ok(RawMatchLike::String(s.as_ref()))
			}
			Expression::TemplateLiteral(t) => {
				Ok(RawMatchLike::Template(t.as_ref()))
			}
			Expression::BinaryExpression(b) => {
				if let Some(cow) = b.evaluate_value_to_string(self) {
					Ok(RawMatchLike::ComputedString(
						self.ast_builder.atom_from_cow(&cow),
						b.span,
					))
				} else {
					bail!("invalid bin exp for match-like");
				}
			}
			_ => bail!("invalid match-like type"),
		}
	}

	fn try_into_raw_replacement<'a: 'ast>(
		&'a self,
		obj: &'ast ObjectExpression<'ast>,
	) -> Result<RawReplacement<'ast>> {
		let match_ = &obj
			.get_property("match")
			.context("replacement missing match")?
			.value;
		let match_ = self.try_into_raw_match_like(match_)?;
		let replace = &obj
			.get_property("replace")
			.context("replacement missing replace")?
			.value;
		let replace = self.try_into_raw_replace(replace)?;
		let no_warn = obj.parse_bool_flag("noWarn")?;
		let predicate = obj
			.get_property("predicate")
			.map(|p| &p.value)
			.into();
		Ok(RawReplacement {
			match_,
			replace,
			no_warn,
			predicate,
		})
	}

	fn try_into_raw_replace<'a: 'ast>(
		&'a self,
		value: &'ast Expression<'ast>,
	) -> Result<RawReplace<'ast>> {
		match value {
			Expression::StringLiteral(s) => Ok(RawReplace::String(s.as_ref())),
			Expression::ArrowFunctionExpression(s) => {
				Ok(RawReplace::Func(s.as_ref()))
			}
			Expression::TemplateLiteral(s) => {
				Ok(RawReplace::Template(s.as_ref()))
			}
			Expression::BinaryExpression(s) => {
				if let Some(cow) = s.evaluate_value_to_string(self) {
					Ok(RawReplace::ComputedString(
						self.ast_builder.atom_from_cow(&cow),
						s.span,
					))
				} else {
					bail!("invalid bin exp for replace");
				}
			}
			_ => bail!("invalid replace type {}", value.dbg_name()),
		}
	}

	fn parse_single_patch<'a: 'ast>(
		&'a self,
		obj: &'ast ArrayExpressionElement<'ast>,
	) -> Result<RawPatch<'ast>> {
		let obj = match obj {
			ArrayExpressionElement::SpreadElement(_) => {
				bail!(
					"Spreads and dynamic expressions are not supported in patches yet."
				)
			}
			ArrayExpressionElement::ObjectExpression(obj) => obj.as_ref(),
			_ => {
				bail!(
					"invalid element in patches array, got: {}",
					obj.dbg_name()
				);
			}
		};

		let all = obj.parse_bool_flag("all")?;
		let no_warn = obj.parse_bool_flag("noWarn")?;
		let predicate = obj
			.get_property("predicate")
			.map(|p| &p.value)
			.into();
		let find = &obj
			.get_property("find")
			.context("patch missing find")?
			.value;

		let find = self.try_into_raw_match_like(find)?;

		let replacement = self.parse_replacement(
			&obj.get_property("replacement")
				.context("patch missing replacement")?
				.value,
		)?;

		let ret = RawPatch {
			all,
			no_warn,
			predicate,
			find,
			replacement,
		};

		Ok(ret)
	}

	fn parse_replacement<'a: 'ast>(
		&'a self,
		prop: &'ast Expression<'ast>,
	) -> Result<OxcVec<'ast, RawReplacement<'ast>>> {
		let ret = match prop {
			Expression::ArrayExpression(arr) => {
				let elements = &arr.elements;
				let mut ret =
					OxcVec::with_capacity_in(elements.len(), self.alloc);
				for elem in elements {
					let elem = elem
						.as_expression()
						.and_then(Expression::as_object_expression)
						.context("invalid replacement type")?;
					ret.push(self.try_into_raw_replacement(elem)?);
				}
				ret
			}
			Expression::ObjectExpression(obj) => OxcVec::from_array_in(
				[self.try_into_raw_replacement(obj.as_ref())?],
				self.alloc,
			),
			_ => bail!("invalid replacement type"),
		};
		Ok(ret)
	}

	fn parse_spread_patch<'a: 'ast>(
		&'a self,
		spread: &'ast SpreadElement<'ast>,
		ret: &mut OxcVec<'ast, RawPatch<'ast>>,
	) -> Option<()> {
		let call = spread.argument.as_call_expression()?;
		if call.arguments.len() != 1 {
			return None;
		}
		let mapper = call.arguments[0]
			.as_expression()?
			.as_arrow_function_expression()?;
		if mapper.params.parameters_count() != 1 || mapper.params.rest.is_some()
		{
			return None;
		}
		let mapper_param = mapper.params.items[0]
			.pattern
			.as_binding_identifier()?
			.symbol_id();
		let map_prop = call.callee.as_member_expression()?;
		if map_prop.static_property_name() != Some("map") {
			return None;
		}
		let arr = map_prop
			.object()
			.as_array_expression()?;
		if arr.elements.is_empty() {
			return None;
		}
		let obj = self
			.get_arrow_single_return_value(mapper)?
			.without_parentheses()
			.as_object_expression()?;
		let find = obj
			.get_property("find")?
			.value
			.as_identifier()?
			.reference_id();

		if self
			.sema
			.scoping()
			.get_reference(find)
			.symbol_id()
			!= Some(mapper_param)
		{
			return None;
		}
		let all = obj.parse_bool_flag("all").ok()?;
		let no_warn = obj.parse_bool_flag("noWarn").ok()?;
		let predicate = obj
			.get_property("predicate")
			.map(|p| &p.value)
			.into();
		let replacement = self
			.parse_replacement(&obj.get_property("replacement")?.value)
			.ok()?;
		ret.reserve(arr.elements.len());
		for e in &arr.elements {
			let find = e.as_expression()?;
			let find = self
				.try_into_raw_match_like(find)
				.ok()?;

			ret.push(RawPatch {
				all,
				no_warn,
				predicate,
				find,
				replacement: OxcVec::from_iter_in(
					replacement.iter().cloned(),
					self.alloc,
				),
			});
		}

		Some(())
	}

	fn get_arrow_single_return_value<'a: 'ast>(
		&'a self,
		func: &'ast ArrowFunctionExpression<'ast>,
	) -> Option<&'ast Expression<'ast>> {
		_ = self;
		// TODO: use CFG to get return value of arrow function that might have a body
		func.get_expression()
	}
	// TODO: skip all && noWarn patches?
	// maybe noop the replace and just test that the find matches at least once
	#[allow(clippy::cognitive_complexity)]
	fn raw_patches<'a: 'ast>(&'a self) -> Result<OxcVec<'ast, RawPatch<'ast>>> {
		let mut ret = OxcVec::new_in(self.alloc);
		let Some(patches) = self
			.define_plugin()
			.and_then(|o| Some(&o.get_property("patches")?.value))
		else {
			trace!("No patches found for plugin");
			return Ok(ret);
		};

		let Expression::ArrayExpression(patches) = patches else {
			bail!("invalid type for patches, expected array");
		};

		let patches = patches.as_ref();

		for patch_obj in &patches.elements {
			if let Some(spread) = patch_obj.as_spread() {
				if self
					.parse_spread_patch(spread, &mut ret)
					.is_none()
				{
					let plugin_name = self.plugin_name();
					warn!(
						"Failed to parse spread patch for plugin {plugin_name:?}, skipping"
					);
				}
			} else {
				match self.parse_single_patch(patch_obj) {
					Ok(patch) => {
						ret.push(patch);
					}
					Err(e) => {
						let plugin_name = self.plugin_name();
						debug!(
							"Failed to parse patch for plugin {plugin_name:?}, skipping. Error: {e:#?}"
						);
					}
				}
			}
		}

		trace!(
			"Parsed {} patches for plugin {:?}",
			ret.len(),
			self.plugin_name()
		);

		Ok(ret)
	}

	pub fn canonicalize_replace_func<'a: 'ast>(
		&self,
		f: &'ast ArrowFunctionExpression<'ast>,
	) -> Result<ReplaceLike> {
		let template_val = self
			.get_arrow_single_return_value(f)
			.context("replace function does not have a single return value")?
			.as_template_literal()
			.context(
				"replace functions only support a template literal return as of now",
			)?;
		let mut parameter_map: OxcHashMap<SymbolId, u8> =
			OxcHashMap::with_capacity_in(f.params.items.len(), self.alloc);
		if f.params.rest.is_some() {
			bail!("replace function has a rest param")
		}
		for (i, param) in f.params.items.iter().enumerate() {
			if param.initializer.is_some() {
				bail!("replace function has a default param")
			}
			let Some(ident) = param.pattern.get_binding_identifier() else {
				bail!(
					"replace function has parameter that is not a plain identifier"
				);
			};
			// should be true, but for sanity
			debug_assert!(param.pattern.is_binding_identifier());
			debug_assert!(u8::try_from(i).is_ok(), "capture group overflow");
			let insert_result =
				parameter_map.insert(ident.symbol_id(), i as u8);
			debug_assert_eq!(
				insert_result, None,
				"should never have duplicate symbol ids"
			);
		}

		let mut ret = TemplateEvaluator {
			lits: Vec::with_capacity(template_val.quasis.len()),
			captures: Vec::with_capacity(template_val.expressions.len()),
		};

		let mut it = template_val.quasis.iter().map(|e| {
			e.value
				.cooked
				// unwrap() here is safe because this is not a tagged template
				// literal; therefore, there will always be a cooked value
				.unwrap()
				.to_string()
		});
		// handle the first one because it has nothing before it
		ret.lits.push(it.next().unwrap());

		for (lit, expr) in it.zip(template_val.expressions.iter()) {
			let ref_id = expr
				.as_identifier()
				.context("Template expr is not an identifier")?
				.reference_id();
			let sym_id = self
				.sema
				.scoping()
				.get_reference(ref_id)
				.symbol_id()
				.context("template expr has unbound ident")?;
			let capture_idx = *parameter_map
				.get(&sym_id)
				.context("template expr uses ident that is not a parameter")?;
			ret.captures.push(capture_idx);
			ret.lits.push(lit);
		}
		assert_eq!(
			ret.lits.len(),
			ret.captures.len() + 1,
			"there should always be one more literal than captures"
		);
		let ret = ReplaceLike {
			v: Replacer::Template(ret),
			s: f.span,
		};

		Ok(ret)
	}

	pub fn canonicalize_patch(&self, raw: RawPatch<'_>) -> Result<Patch> {
		let all = raw.all;
		let no_warn = raw.no_warn;
		let find = canonicalize_match_like(&raw.find)?;
		let mut replacement = Vec::with_capacity(raw.replacement.len());

		for r in raw.replacement {
			let match_ = canonicalize_match_like(&r.match_)?;
			let no_warn = r.no_warn;
			let replace = match &r.replace {
				RawReplace::String(StringLiteral { value, span, .. })
				| RawReplace::ComputedString(value, span) => {
					let mut value = value.to_string();
					canonicalize_replace_for_regress(&mut value);
					ReplaceLike {
						v: Replacer::Str(value),
						s: *span,
					}
				}
				RawReplace::Func(f) => self.canonicalize_replace_func(f)?,
				RawReplace::Template(_) => {
					bail!("Template literal replacements are not supported yet")
				}
			};

			replacement.push(Replacement {
				match_,
				replace,
				no_warn,
			});
		}

		Ok(Patch {
			all,
			no_warn,
			find,
			replacement,
			plugin_id: None,
		})
	}
	pub fn patches(&self) -> Result<Vec<Patch>> {
		let name = self
			.plugin_name()
			.unwrap_or("<unknown plugin>");
		let ret = self
			.raw_patches()?
			.into_iter()
			.filter_map(|raw| match self.canonicalize_patch(raw) {
				Ok(patch) => Some(patch),
				Err(e) => {
					debug!(
						"Failed to canonicalize patch for plugin {name}, skipping. Cause: {e:?}"
					);
					None
				}
			})
			.collect();
		Ok(ret)
	}
}

impl GlobalContext<'_> for VencordAstParser<'_> {
	fn is_global_reference(
		&self,
		_reference: &oxc::ast::ast::IdentifierReference<'_>,
	) -> bool {
		false
	}
}

impl MayHaveSideEffectsContext<'_> for VencordAstParser<'_> {
	fn annotations(&self) -> bool {
		true
	}

	fn manual_pure_functions(&self, _callee: &Expression) -> bool {
		false
	}

	fn property_read_side_effects(&self) -> PropertyReadSideEffects {
		PropertyReadSideEffects::None
	}

	fn unknown_global_side_effects(&self) -> bool {
		false
	}
}

impl<'ast> ConstantEvaluationCtx<'ast> for VencordAstParser<'ast> {
	fn ast(&self) -> AstBuilder<'ast> {
		self.ast_builder
	}
}

impl<'ast> AstParser<'ast> for VencordAstParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}
}

impl<'ast> ESModuleParser<'ast> for VencordAstParser<'ast> {}
