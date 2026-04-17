use std::sync::mpsc;

use crate::{
	Patch, ReplaceLike, Replacement, Replacer, TemplateEvaluator, diag::{LocalSource, PResult, ParserDiagnostic, err, err_ns}, pass::{
		EvalStringRawPass,
		FlattenTemplatePass,
		FoldBinaryExpressionsPass,
		InlineConstantsPass,
		InlineEnumsPass,
		PassManager,
	}, patches::{
		RawMatchLike,
		RawPatch,
		RawReplace,
		RawReplacement,
		canonicalize_match_like,
		canonicalize_replace_for_regress,
	}
};
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
			StringLiteral, TemplateLiteral,
		},
	},
	diagnostics::OxcDiagnostic,
	minifier::PropertyReadSideEffects,
	semantic::{Semantic, SymbolId},
	span::{GetSpan, SourceType},
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
	pub(crate) txt: &'ast str,
	pub(crate) path: &'ast str,
	pub diag_ch: Option<mpsc::Sender<ParserDiagnostic>>,
}

const DEFINE_PLUGIN_IMPORT_SOURCE: &str = "@utils/types";

// TODO: get webpack finds
impl<'ast> VencordAstParser<'ast> {
	pub fn try_new(alloc: &'ast Allocator, source: &'ast str, path: Option<&'ast str>) -> PResult<Self> {
		let pass_data =
			match parse_for_traverse(alloc, source, SourceType::tsx()) {
				Ok(d) => d,
				Err(e) => {
					let mut err = err_ns("Failed to parse source");
					if e.is::<OxcDiagnostic>() {
						let oxc_diag = e.downcast::<OxcDiagnostic>().unwrap();
						err = err.s(oxc_diag);
					} else {
						err = err.s(miette::miette!(e));
					}
					return Err(err);
				}
			};

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
			txt: source,
			path: path.unwrap_or("file.tsx"),
			diag_ch: None,
		})
	}

	pub fn collect_diagnostics(&self) {
		debug_assert!(self.diag_ch.is_some(), "diag_ch should be set before calling collect_diagnostics");
		if let Err(e) = self.raw_patches() {
			_ = self.diag_ch.as_ref().unwrap().send(e);
		}
	}

	fn define_plugin<'a: 'ast>(
		&'a self,
	) -> PResult<&'ast ObjectExpression<'ast>> {
		let utils_types_import = self
			.find_import_by_name(DEFINE_PLUGIN_IMPORT_SOURCE)
			.ok_or_else(|| err_ns("Failed to find `@utils/types` import"))?;
		let define_plugin = utils_types_import
			.default_var()
			.ok_or_else(|| {
				err(
					utils_types_import,
					"No default import used from `@utils/types` for definePlugin",
				)
			})?;
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
			return Ok(obj.as_ref());
		}
		Err(err_ns("Failed to find definePlugin call"))
	}

	fn plugin_name<'a: 'ast>(&'a self) -> PResult<&'ast str> {
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e))?;
		let name_prop = define_plugin
			.get_property("name")
			.ok_or_else(|| {
				err(define_plugin, "definePlugin does not have `name` prop")
			})?;
		let name_val = name_prop
			.value
			.as_string_literal()
			.ok_or_else(|| {
				err(&name_prop.value, "`name` is not a string literal")
			})?
			.value
			.as_str();
		Ok(name_val)
	}

	fn try_into_raw_match_like<'a: 'ast>(
		&'a self,
		value: &'ast Expression<'ast>,
	) -> PResult<RawMatchLike<'ast>> {
		match value {
			Expression::RegExpLiteral(r) => Ok(RawMatchLike::Regex(r.as_ref())),
			Expression::StringLiteral(s) => {
				Ok(RawMatchLike::String(s.as_ref()))
			}
			Expression::TemplateLiteral(t) => {
				Ok(RawMatchLike::Template(t.as_ref()))
			}
			Expression::BinaryExpression(b) => {
				let cow = b
					.evaluate_value_to_string(self)
					.ok_or_else(|| {
						err(
							b.as_ref(),
							"Invalid binary expression for match-like",
						)
					})?;
				Ok(RawMatchLike::ComputedString(
					self.ast_builder.str_from_cow(&cow),
					b.span,
				))
			}
			_ => Err(err(value, "Invalid match-like type")),
		}
	}

	fn try_into_raw_replacement<'a: 'ast>(
		&'a self,
		obj: &'ast ObjectExpression<'ast>,
	) -> PResult<RawReplacement<'ast>> {
		let match_ = &obj
			.get_property("match")
			.ok_or_else(|| err(obj, "replacement missing match"))?
			.value;
		let match_ = self.try_into_raw_match_like(match_)?;
		let replace = &obj
			.get_property("replace")
			.ok_or_else(|| err(obj, "replacement missing replace"))?
			.value;
		let replace = self
			.try_into_raw_replace(replace)
			.map_err(|e| err(replace, "Failed to parse replacement").s(e))?;
		let no_warn = obj
			.parse_bool_flag("noWarn")
			.map_err(|e| err(e, "noWarn prop is not a boolean"))?;
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
	) -> PResult<RawReplace<'ast>> {
		match value {
			Expression::StringLiteral(s) => Ok(RawReplace::String(s.as_ref())),
			Expression::ArrowFunctionExpression(s) => {
				Ok(RawReplace::Func(s.as_ref()))
			}
			Expression::TemplateLiteral(s) => {
				Ok(RawReplace::Template(s.as_ref()))
			}
			Expression::BinaryExpression(s) => {
				let cow = s
					.evaluate_value_to_string(self)
					.ok_or_else(|| {
						err(s.as_ref(), "Invalid bin exp for replace")
					})?;
				Ok(RawReplace::ComputedString(
					self.ast_builder.str_from_cow(&cow),
					s.span,
				))
			}
			_ => Err(err(value, "Invalid replace type")),
		}
	}

	fn parse_single_patch<'a: 'ast>(
		&'a self,
		obj: &'ast ArrayExpressionElement<'ast>,
	) -> PResult<RawPatch<'ast>> {
		let obj = match obj {
			ArrayExpressionElement::SpreadElement(e) => {
				return Err(err(
					e.as_ref(),
					"Spreads and dynamic expressions are not supported in patches yet.",
				));
			}
			ArrayExpressionElement::ObjectExpression(obj) => obj.as_ref(),
			_ => {
				return Err(err(obj, "invalid element in patches array"));
			}
		};

		let all = obj
			.parse_bool_flag("all")
			.map_err(|e| err(e, "all prop is not a boolean literal"))?;
		let no_warn = obj
			.parse_bool_flag("noWarn")
			.map_err(|e| err(e, "noWarn prop is not a boolean literal"))?;
		let predicate = obj
			.get_property("predicate")
			.map(|p| &p.value)
			.into();
		let find = &obj
			.get_property("find")
			.ok_or_else(|| err(obj, "patch missing find property"))?
			.value;

		let find = self.try_into_raw_match_like(find)?;

		let replacement = self.parse_replacement(
			&obj.get_property("replacement")
				.ok_or_else(|| err(obj, "patch missing replacement"))?
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
	) -> PResult<OxcVec<'ast, RawReplacement<'ast>>> {
		let ret = match prop {
			Expression::ArrayExpression(arr) => {
				let elements = &arr.elements;
				let mut ret =
					OxcVec::with_capacity_in(elements.len(), self.alloc);
				for elem in elements {
					let elem = elem
						.as_expression()
						.and_then(Expression::as_object_expression)
						.ok_or_else(|| err(elem, "invalid replacement type"))?;
					ret.push(
						self.try_into_raw_replacement(elem)
							.map_err(|e| {
								err_ns("Failed to parse replacement").s(e)
							})?,
					);
				}
				ret
			}
			Expression::ObjectExpression(obj) => OxcVec::from_array_in(
				[self.try_into_raw_replacement(obj.as_ref())?],
				self.alloc,
			),
			_ => return Err(err(prop, "invalid replacement type")),
		};
		Ok(ret)
	}

	/// TODO: better error handling
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
	fn raw_patches<'a: 'ast>(
		&'a self,
	) -> PResult<OxcVec<'ast, RawPatch<'ast>>> {
		let mut ret = OxcVec::new_in(self.alloc);
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e))?;
		let Some(patches) = define_plugin
			.get_property("patches")
			.map(|p| &p.value)
		else {
			trace!("No patches found for plugin");
			return Ok(ret);
		};

		let patches = patches
			.as_array_expression()
			.ok_or_else(|| {
				err(patches, "Expected patches to be an array literal")
			})?;

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
						let e = miette::Report::from(e);
						let e = LocalSource {
							name: self.path,
							source: self.txt,
							inner: e,
						};
						debug!(
							"Failed to parse patch, skipping. Cause:\n{e:?}"
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
	) -> PResult<ReplaceLike> {
		let f_ret = self
			.get_arrow_single_return_value(f)
			.ok_or_else(|| {
				err(
					f.body.as_ref(),
					"replace function does not have a single return value",
				)
			})?;
		let template_val = f_ret
			.as_template_literal()
			.ok_or_else(|| {
				err(
					f_ret,
					"replace functions only support a template literal return as of now",
				)
			})?;
		let mut parameter_map: OxcHashMap<SymbolId, u8> =
			OxcHashMap::with_capacity_in(f.params.items.len(), self.alloc);
		if let Some(r) = f.params.rest.as_deref() {
			return Err(err(
				r,
				"replace function cannot have a rest parameter",
			));
		}
		for (i, param) in f.params.items.iter().enumerate() {
			if let Some(e) = param.initializer.as_deref() {
				return Err(err(e, "replace function has a default param"));
			}
			let Some(ident) = param.pattern.get_binding_identifier() else {
				return Err(err(
					&param.pattern,
					"replace function has parameter that is not a plain identifier",
				));
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
			let ident_ref = expr.as_identifier().ok_or_else(|| {
				err(expr, "Template expr is not an identifier")
			})?;
			let ref_id = ident_ref.reference_id();
			let sym_id = self
				.sema
				.scoping()
				.get_reference(ref_id)
				.symbol_id()
				.ok_or_else(|| {
					err(ident_ref, "template expr has an unbound ident")
				})?;
			let capture_idx = *parameter_map
				.get(&sym_id)
				.ok_or_else(|| {
					err(
						ident_ref,
						"template expr uses ident that is not a parameter",
					)
				})?;
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

	pub fn canonicalize_patch(&self, raw: RawPatch<'_>) -> PResult<Patch> {
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
				RawReplace::Template(TemplateLiteral {span, ..}) => {
					return Err(err(
						span,
						"Template literal replacements are not supported yet",
					));
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
	pub fn patches(&self) -> PResult<Vec<Patch>> {
		let name = self
			.plugin_name()
			.unwrap_or("<unknown plugin>");
		let ret = self
			.raw_patches().map_err(|e| err_ns("Failed to parse raw patches").s(e))?
			.into_iter()
			.filter_map(|raw| match self.canonicalize_patch(raw) {
				Ok(patch) => Some(patch),
				Err(e) => {
					let e = LocalSource {
						name: self.path,
						source: self.txt,
						inner: miette::Report::from(e),
					};
					debug!(
						"Failed to canonicalize patch for plugin {name}, skipping. Cause:\n{e:?}"
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
