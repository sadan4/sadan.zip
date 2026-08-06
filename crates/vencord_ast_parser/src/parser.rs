mod collect_capture_groups;
pub(crate) use collect_capture_groups::{
	Capture,
	GroupInfo,
	GroupReference,
	collect_capture_groups,
};
use memchr::memmem::Finder;
use smol_str::{SmolStr, ToSmolStr};

use std::{collections::HashMap, sync::mpsc, vec};

use crate::{
	AnyFindType,
	FindArg,
	FindData,
	FindType,
	FindUse,
	Match,
	MatchLike,
	MatchRegex,
	Patch,
	PluginInfo,
	ReplaceLike,
	Replacement,
	Replacer,
	TemplateEvaluator,
	diag::{LocalSource, PResult, ParserDiagnostic, err, err_ns},
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
		canonicalize_intl,
		canonicalize_regex_ident,
		canonicalize_replace_for_regress,
	},
	types::{Dev, PluginDev},
};
use ast_parser::{
	AstParser,
	ESModuleParser,
	NodeLocationIndex,
	cache,
	exts::{
		ArrayExpressionElementExt as _,
		BindingPatternExt as _,
		ExpressionExt as _,
		ImportDeclarationExt as _,
		MemberExpressionExt,
		ObjectExpressionExt as _,
		PropertyKeyExt,
	},
	parse_for_traverse,
	sym_id::GetSymId,
};
use oxc::{
	allocator::{
		Allocator,
		GetAllocator,
		HashMap as OxcHashMap,
		Vec as OxcVec,
	},
	ast::{
		AstKind,
		ast::{
			Argument,
			ArrayExpressionElement,
			ArrowFunctionExpression,
			CallExpression,
			Expression,
			ImportDeclarationSpecifier,
			ObjectExpression,
			Program,
			RegExpFlags,
			RegExpLiteral,
			SpreadElement,
			Statement,
			Str,
			StringLiteral,
			TemplateLiteral,
		},
		builder::{AstBuilder, GetAstBuilder},
	},
	minifier::PropertyReadSideEffects,
	semantic::{Semantic, SymbolId},
	span::{GetSpan, SourceType, Span},
};
use oxc_ecmascript::{
	GlobalContext,
	constant_evaluation::{ConstantEvaluation, ConstantEvaluationCtx},
	side_effects::MayHaveSideEffectsContext,
};
use tracing::{debug, trace, warn};

pub struct VencordAstParser<'ast> {
	pub(crate) alloc: &'ast Allocator,
	pub(crate) prog: &'ast Program<'ast>,
	pub(crate) sema: Semantic<'ast>,
	pub(crate) txt: &'ast str,
	pub(crate) path: &'ast str,
	ast_builder: AstBuilder<'ast>,
	cache: Cache<'ast>,
	pub diag_ch: Option<mpsc::Sender<ParserDiagnostic>>,
}

#[derive(Default)]
struct Cache<'ast> {
	finds: cache::Ref<PResult<Vec<FindUse>>>,
	define_plugin: cache::Ref<PResult<&'ast ObjectExpression<'ast>>>,
	node_index: cache::Ref<NodeLocationIndex<'ast>>,
}

const DEFINE_PLUGIN_IMPORT_SOURCE: &str = "@utils/types";
const FIND_IMPORT_SOURCE: &str = "@webpack";

// TODO: get webpack finds
/// Public API
impl<'ast> VencordAstParser<'ast> {
	pub fn try_new(
		alloc: &'ast Allocator,
		source: &'ast str,
		path: Option<&'ast str>,
	) -> Result<Self, miette::Error> {
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
		let ast_builder = AstBuilder::new(alloc);
		Ok(Self {
			alloc,
			prog,
			sema,
			txt: source,
			path: path.unwrap_or("file.tsx"),
			ast_builder,
			cache: Cache::default(),
			diag_ch: None,
		})
	}
	pub fn collect_diagnostics(&self) {
		debug_assert!(
			self.diag_ch.is_some(),
			"diag_ch should be set before calling collect_diagnostics"
		);
		let patches = match self.raw_patches() {
			Err(e) => {
				self.diag_ch
					.as_ref()
					.unwrap()
					.send(e)
					.ok();
				return;
			}
			Ok(e) => e,
		};
		let plugin_keys = self
			.top_level_plugin_keys()
			.unwrap_or_default();
		for patch in patches {
			self.check_find(patch.find);
			for repl in &patch.replacement {
				self.check_replacement(repl);
				self.check_self_references(repl, &plugin_keys);
			}
		}
	}

	/// The plugin's name and the source span of its `definePlugin({...})`
	/// object literal. Returns `Err(reason)` if the file doesn't look like a
	/// vencord plugin (no `@utils/types` import, no `definePlugin` call,
	/// or no `name` string property).
	pub fn plugin_info(&self) -> PResult<PluginInfo> {
		let name = self.plugin_name()?.to_smolstr();
		// name requires define_plugin, so this should never error
		let span = self.define_plugin().unwrap().span;
		let description = self
			.plugin_desc()?
			.map(ToSmolStr::to_smolstr);
		let devs = self.plugin_devs()?;
		let top_level_plugin_keys = self.top_level_plugin_keys()?;
		Ok(PluginInfo {
			name,
			description,
			devs,
			top_level_plugin_keys,
			span,
		})
	}

	/// Walk all `definePlugin({ patches })` and return the canonical patches.
	/// See [`Self::canonicalize_patch`] for the meaning of `apply_regress_canon`.
	pub fn patches(&self, apply_regress_canon: bool) -> PResult<Vec<Patch>> {
		let name = self
			.plugin_name()
			.unwrap_or("<unknown plugin>");
		let ret = self
			.raw_patches()
			.map_err(|e| err_ns("Failed to parse raw patches").s(e))?
			.into_iter()
			.filter_map(|raw| {
				match self.canonicalize_patch(raw, apply_regress_canon) {
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
				}
			})
			.collect();
		Ok(ret)
	}

	pub fn get_finds(&self) -> &PResult<Vec<FindUse>> {
		self.cache
			.finds
			.get(|| self.get_finds_())
	}

	/// If `offset` falls on a `$self.<prop>` reference inside a string
	/// replacement, returns the span of `<prop>`'s definition — its key in the
	/// `definePlugin({...})` object. Returns `None` when the cursor isn't on
	/// such a reference, or when `<prop>` is not a known top-level plugin key
	/// (an unbound reference, which [`Self::check_self_references`] flags).
	///
	/// Powers "go to definition" on `$self.<prop>` in the LSP.
	pub fn self_reference_definition(&self, offset: u32) -> Option<Span> {
		let plugin_keys = self.top_level_plugin_keys().ok()?;
		let patches = self.raw_patches().ok()?;
		for patch in &patches {
			for repl in &patch.replacement {
				let (value, scan_offset) = match repl.replace {
					RawReplace::ComputedString(value, span) => {
						(value.as_str(), span.start + 1)
					}
					RawReplace::String(lit) => {
						(lit.value.as_str(), lit.span.start + 1)
					}
					// In a function or template replacement, `$self.<prop>`
					// appears verbatim in the raw source (it's a textual
					// substitution Vencord applies to the stringified replace),
					// so scan the node's source slice with its start as the
					// offset — no `+ 1` since there's no opening quote to skip.
					RawReplace::Func(f) => (
						&self.txt[f.span.start as usize..f.span.end as usize],
						f.span.start,
					),
					RawReplace::Template(t) => (
						&self.txt[t.span.start as usize..t.span.end as usize],
						t.span.start,
					),
				};
				let refs = Self::collect_self_references(value, scan_offset);
				for (prop, spans) in &refs {
					if spans
						.iter()
						.any(|s| (s.start..=s.end).contains(&offset))
					{
						return plugin_keys.get(prop).copied();
					}
				}
			}
		}
		None
	}
}
/// Diagnostics
#[expect(clippy::multiple_inherent_impl)]
impl<'ast> VencordAstParser<'ast> {
	fn unused_capture_group(capture: &Capture) -> ParserDiagnostic {
		let extra_label = capture
		.group
		.name
		.map_or_else(|| {
			let start_span = Span::new(
				capture.group.span.start + 1,
				capture.group.span.start + 1,
			);
			(start_span, "Consider inserting `?:` here to make this a non-capturing group".into())
		}, |name| {
			let group_syntax_span = Span::new(
				capture.group.span.start + 1,
				capture.group.span.start + 1 + 1 + name.len() as u32 + 1,
			);
			(group_syntax_span, "Consider replacing this with `?:` to make it a non-capturing group".into())
		});
		ParserDiagnostic {
			msg: "Unused capture group".into(),
			labels: vec![extra_label],
			severity: miette::Severity::Warning,
			..Default::default()
		}
	}

	fn check_find(&self, find: RawMatchLike<'ast>) {
		if let RawMatchLike::Regex(r) = find {
			let ch = self.diag_ch.as_ref().unwrap();
			let Some(pat) = r.regex.pattern.pattern.as_deref() else {
				let diag = ParserDiagnostic {
					msg: "Regex pattern could not be parsed".into(),
					labels: vec![(
						r.span,
						"Skipping regex-specific lint checks for this find."
							.into(),
					)],
					severity: miette::Severity::Warning,
					..Default::default()
				};
				ch.send(diag).ok();
				return;
			};
			let group_info = collect_capture_groups(self, pat);
			if r.regex.flags.contains(RegExpFlags::G) {
				let diag = ParserDiagnostic {
					msg: "Using the global flag in a find has no effect".into(),
					// TODO: better label span
					labels: vec![(
						r.span,
						"Consider setting `all: true` on the patch, which was probably the original intent.".into(),
					)],
					severity: miette::Severity::Warning,
					..Default::default()
				};
				ch.send(diag).ok();
			}
			for unbound_ref in &group_info.unbound_refs {
				ch.send(err(unbound_ref, "Unbound backreference"))
					.ok();
			}
			for capture in &group_info.indexed_groups {
				if capture.refs.is_empty() {
					ch.send(Self::unused_capture_group(capture))
						.ok();
					continue;
				}
				if let Some(name) = capture.group.name {
					for r in &capture.refs {
						let GroupReference::Indexed(r) = *r else {
							continue;
						};
						let diag = ParserDiagnostic {
							msg: format!(
								"Named capture group `{name}` used in an indexed backreference"
							)
							.into(),
							labels: vec![
								(
									capture.group.span,
									format!("Group `{name}` declared here")
										.into(),
								),
								(
									r.span,
									format!(
										"Consider replacing this with `\\k<{name}>`"
									)
									.into(),
								),
							],
							severity: miette::Severity::Warning,
							..Default::default()
						};
						ch.send(diag).ok();
					}
				}
			}
		}
	}

	/// Errors on any `$self.<prop>` reference in a string replacement whose
	/// `<prop>` is not a top-level key of the plugin. Unlike
	/// [`Self::check_replacement`], this is not gated on the match being a
	/// regex — `$self` is valid for any find.
	fn check_self_references(
		&self,
		repl: &RawReplacement<'ast>,
		plugin_keys: &HashMap<SmolStr, Span>,
	) {
		let (value, replace_span) = match repl.replace {
			RawReplace::ComputedString(value, span) => (value.as_str(), span),
			RawReplace::String(lit) => (lit.value.as_str(), lit.span),
			RawReplace::Func(_) | RawReplace::Template(_) => return,
		};

		let ch = self.diag_ch.as_ref().unwrap();
		let refs = Self::collect_self_references(value, replace_span.start + 1);
		for (prop, spans) in &refs {
			if plugin_keys.contains_key(prop) {
				continue;
			}
			for &span in spans {
				ch.send(ParserDiagnostic {
					msg: format!(
						"`$self.{prop}` references `{prop}`, which is not a top-level property of the plugin"
					)
					.into(),
					labels: vec![(
						span,
						"no such property on the plugin".into(),
					)],
					severity: miette::Severity::Error,
					..Default::default()
				})
				.ok();
			}
		}
	}

	fn check_replacement(&self, repl: &RawReplacement<'ast>) {
		let RawMatchLike::Regex(lit) = repl.match_ else {
			return;
		};

		let Some((value, replace_span)) = (match repl.replace {
			RawReplace::ComputedString(value, span) => {
				Some((value.as_str(), span))
			}
			RawReplace::String(string_lit) => {
				Some((string_lit.value.as_str(), string_lit.span))
			}
			RawReplace::Func(_) | RawReplace::Template(_) => None,
		}) else {
			return;
		};

		let captures = collect_capture_groups(
			self,
			lit.regex
				.pattern
				.pattern
				.as_deref()
				.unwrap(),
		);

		self.check_replacement_string(value, replace_span, lit.span, &captures);
	}

	fn check_replacement_string(
		&self,
		value: &str,
		replace_span: Span,
		regex_span: Span,
		captures: &GroupInfo<'ast>,
	) {
		let mut visited_captures =
			Vec::with_capacity(captures.indexed_groups.len());
		let mut it = value.chars().enumerate().peekable();

		while let Some((start_idx, c)) = it.next() {
			if c != '$' {
				continue;
			}

			let Some((_, marker)) = it.peek().copied() else {
				continue;
			};

			match marker {
				'$' => {
					it.next();
				}
				'1'..='9' => {
					let mut group_num = 0usize;
					let mut end_idx = start_idx;
					while let Some(&(idx, digit)) = it.peek() {
						if !digit.is_ascii_digit() {
							break;
						}
						it.next();
						group_num *= 10;
						group_num += (digit as u32 - '0' as u32) as usize;
						end_idx = idx;
						if group_num > u16::MAX as usize {
							break;
						}
					}

					let capture_idx = group_num - 1;
					visited_captures.push(capture_idx);
					self.check_indexed_replacement_reference(
						group_num,
						start_idx,
						end_idx,
						replace_span,
						regex_span,
						captures.indexed_groups.len(),
					);
				}
				'<' => {
					it.next();
					let name: String = it
						.by_ref()
						.map_while(|(_, c)| (c != '>').then_some(c))
						.collect();
					if let Some(capture_idx) = self
						.check_named_replacement_reference(
							name.as_str(),
							start_idx,
							replace_span,
							regex_span,
							captures,
						) {
						visited_captures.push(capture_idx);
					}
				}
				_ => {}
			}
		}

		self.warn_unused_replacement_captures(captures, &visited_captures);
	}

	fn check_indexed_replacement_reference(
		&self,
		group_num: usize,
		start_idx: usize,
		end_idx: usize,
		replace_span: Span,
		regex_span: Span,
		capture_count: usize,
	) {
		if group_num <= capture_count {
			return;
		}

		let diag = ParserDiagnostic {
			msg: "Replace references a non-existent capture group".into(),
			labels: vec![
				(
					Span::new(
						replace_span.start + start_idx as u32 + 1,
						replace_span.start + end_idx as u32 + 2,
					),
					format!("Group {group_num} referenced here").into(),
				),
				(
					regex_span,
					format!(
						"Only {capture_count} capture groups declared here"
					)
					.into(),
				),
			],
			..Default::default()
		};
		self.diag_ch
			.as_ref()
			.unwrap()
			.send(diag)
			.ok();
	}

	fn check_named_replacement_reference(
		&self,
		name: &str,
		start_idx: usize,
		replace_span: Span,
		regex_span: Span,
		captures: &GroupInfo<'ast>,
	) -> Option<usize> {
		let group_idx = captures
			.indexed_groups
			.iter()
			.position(|capture| {
				capture.group.name.map(|n| n.as_str()) == Some(name)
			});

		if group_idx.is_some() {
			return group_idx;
		}

		let diag = ParserDiagnostic {
			msg: format!(
				"Replace references non-existent capture group `{name}`"
			)
			.into(),
			labels: vec![
				(
					Span::new(
						replace_span.start + start_idx as u32 + 1,
						replace_span.start
							+ start_idx as u32 + 1
							+ name.len() as u32,
					),
					format!("Group `{name}` referenced here").into(),
				),
				(
					regex_span,
					format!("No capture group named `{name}` declared here")
						.into(),
				),
			],
			..Default::default()
		};
		self.diag_ch
			.as_ref()
			.unwrap()
			.send(diag)
			.ok();
		None
	}

	fn warn_unused_replacement_captures(
		&self,
		captures: &GroupInfo<'ast>,
		visited_captures: &[usize],
	) {
		for (idx, capture) in captures
			.indexed_groups
			.iter()
			.enumerate()
		{
			if visited_captures.contains(&idx) || !capture.refs.is_empty() {
				continue;
			}

			self.diag_ch
				.as_ref()
				.unwrap()
				.send(Self::unused_capture_group(capture))
				.ok();
		}
	}
}

#[expect(clippy::multiple_inherent_impl)]
/// Private API
impl<'ast> VencordAstParser<'ast> {
	fn define_plugin_impl(&self) -> PResult<&'ast ObjectExpression<'ast>> {
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

	fn define_plugin(
		&self,
	) -> Result<&'ast ObjectExpression<'ast>, &ParserDiagnostic> {
		match self
			.cache
			.define_plugin
			.get(|| self.define_plugin_impl())
		{
			Ok(a) => Ok(*a),
			Err(e) => Err(e),
		}
	}

	fn plugin_name(&self) -> PResult<&'ast str> {
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e.clone()))?;
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

	fn plugin_desc(&self) -> PResult<Option<&'ast str>> {
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e.clone()))?;
		let Some(desc_prop) = define_plugin.get_property("description") else {
			return Ok(None);
		};
		let desc_val = desc_prop
			.value
			.as_string_literal()
			.ok_or_else(|| {
				err(&desc_prop.value, "`description` is not a string literal")
			})?
			.value
			.as_str();
		Ok(Some(desc_val))
	}

	fn parse_dev_el(
		dev: &'ast ArrayExpressionElement<'ast>,
	) -> PResult<PluginDev> {
		match dev {
			ArrayExpressionElement::ObjectExpression(obj) => {
				let obj = obj.as_ref();
				let name_prop = obj
					.get_property("name")
					.ok_or_else(|| {
						err(obj, "Plugin dev object missing `name` property")
					})?;
				let id_prop = obj.get_property("id").ok_or_else(|| {
					err(obj, "Plugin dev object missing `id` property")
				})?;
				let name_val = name_prop
					.value
					.as_string_literal()
					.ok_or_else(|| {
						err(&name_prop.value, "`name` is not a string literal")
					})?
					.value
					.to_smolstr();
				let id_val = id_prop
					.value
					.as_big_int_literal()
					.ok_or_else(|| {
						err(&id_prop.value, "`id` is not a big int literal")
					})?
					.value
					.parse()
					.map_err(|e| {
						err(&id_prop.value, "`id` is not a valid u64")
							.s(miette::Report::msg(e))
					})?;
				Ok(PluginDev {
					dev: Dev::Inline {
						name: name_val,
						id: id_val,
					},
					span: obj.span,
				})
			}
			ArrayExpressionElement::StaticMemberExpression(access) => {
				let access = access.as_ref();
				let key = access.property.name.to_smolstr();
				let obj = access
					.object
					.as_identifier()
					.ok_or_else(|| {
						err(
							&access.object,
							"Object in plugin dev static member expression is not an identifier",
						)
					})?
					.name
					.to_smolstr();
				Ok(PluginDev {
					dev: Dev::Reference { key, obj },
					span: access.span,
				})
			}
			_ => Err(err(
				dev,
				"Invalid plugin dev element. Expected either an (object or static member) expression",
			)),
		}
	}

	fn plugin_devs(&self) -> PResult<Option<Vec<PluginDev>>> {
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e.clone()))?;
		let Some(devs_prop) = define_plugin.get_property("authors") else {
			return Ok(None);
		};
		let devs_arr = devs_prop
			.value
			.as_array_expression()
			.ok_or_else(|| {
				err(&devs_prop.value, "`authors` is not an array literal")
			})?;
		let mut devs = Vec::with_capacity(devs_arr.elements.len());
		for dev in &devs_arr.elements {
			let dev = Self::parse_dev_el(dev)
				.map_err(|e| err_ns("Failed to parse plugin dev").s(e))?;
			devs.push(dev);
		}
		Ok(Some(devs))
	}

	fn top_level_plugin_keys(&self) -> PResult<HashMap<SmolStr, Span>> {
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e.clone()))?;
		let mut ret = HashMap::with_capacity(define_plugin.properties.len());
		for prop in &define_plugin.properties {
			if let Some(key) = prop
				.as_property()
				.and_then(|p| p.key.as_static_identifier())
			{
				ret.insert(key.name.to_smolstr(), key.span);
			} else {
				tracing::debug!(
					"Skipping non-identifier top-level plugin key at {:?}",
					prop.span()
				);
			}
		}
		Ok(ret)
	}

	fn try_into_raw_match_like(
		&self,
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
					Str::from_cow_in(&cow, self),
					b.span,
				))
			}
			_ => Err(err(value, "Invalid match-like type")),
		}
	}

	fn try_into_raw_replacement(
		&self,
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

	fn try_into_raw_replace(
		&self,
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
					Str::from_cow_in(&cow, self),
					s.span,
				))
			}
			_ => Err(err(value, "Invalid replace type")),
		}
	}

	fn parse_single_patch(
		&self,
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
			span: obj.span,
		};

		Ok(ret)
	}

	fn parse_replacement(
		&self,
		prop: &'ast Expression<'ast>,
	) -> PResult<OxcVec<'ast, RawReplacement<'ast>>> {
		let ret = match prop {
			Expression::ArrayExpression(arr) => {
				let elements = &arr.elements;
				let mut ret = OxcVec::with_capacity_in(elements.len(), self);
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
				self,
			),
			_ => return Err(err(prop, "invalid replacement type")),
		};
		Ok(ret)
	}

	/// TODO: refactor to support more types of spread expressions
	#[expect(clippy::too_many_lines)]
	fn parse_spread_patch(
		&self,
		spread: &'ast SpreadElement<'ast>,
		ret: &mut OxcVec<'ast, RawPatch<'ast>>,
	) -> PResult<()> {
		let call = spread
			.argument
			.as_call_expression()
			.ok_or_else(|| {
				err(
					&spread.argument,
					"TODO: support non-call spread expressions",
				)
			})?;
		if call.arguments.len() != 1 {
			return Err(err(
				&spread.argument,
				"Expected exactly one argument to spread expression call",
			));
		}
		let mapper = call.arguments[0]
			.as_expression()
			.ok_or_else(|| {
				err(&call.arguments[0], "call argument is not an expression")
			})?
			.as_arrow_function_expression()
			.ok_or_else(|| {
				err(
					&call.arguments[0],
					"call argument is not an arrow function expression",
				)
			})?;
		if mapper.params.parameters_count() != 1 || mapper.params.rest.is_some()
		{
			return Err(err(
				&spread.argument,
				"Expected exactly one non-rest parameter in arrow map function",
			));
		}
		let mapper_param = &mapper.params.items[0].pattern;
		let mapper_param = mapper_param
			.as_binding_identifier()
			.ok_or_else(|| {
				err(
					mapper_param,
					"Expected map function parameter to be a binding identifier",
				)
			})?;
		let mapper_param_sym_id = mapper_param.symbol_id();
		let map_prop = call
			.callee
			.as_static_member_expression()
			.ok_or_else(|| {
				err(&call.callee, "callee is not a static member expression")
			})?;
		if map_prop.property.name != "map" {
			return Err(err(
				&map_prop.property,
				"callee is not a `.map` static member expression",
			));
		}
		let arr = map_prop
			.object
			.as_array_expression()
			.ok_or_else(|| {
				err(&map_prop.object, "Expected array literal expression")
			})?;
		if arr.elements.is_empty() {
			let err = err(arr, "Empty array literal in spread patch, skipping");
			let inner = miette::Report::from(err);
			let report = LocalSource {
				inner,
				source: self.txt,
				name: self.path,
			};
			tracing::warn!("{report:?}");
			return Ok(());
		}
		let mapper_ret = Self::get_arrow_single_return_value(mapper)
			.ok_or_else(|| {
				err(
					mapper,
					"arrow function does not have a single return value",
				)
			})?
			.without_parentheses();
		let obj = mapper_ret
			.as_object_expression()
			.ok_or_else(|| {
				err(
					mapper_ret,
					"arrow function return value is not an object expression",
				)
			})?;
		let find = &obj
			.get_property("find")
			.ok_or_else(|| err(obj, "patch object missing `find` property"))?
			.value;
		let find = find
			.as_identifier()
			.ok_or_else(|| err(find, "find prop value is not an identifier"))?;
		let find_ref_id = find.reference_id();

		if self
			.sema
			.scoping()
			.get_reference(find_ref_id)
			.symbol_id()
			!= Some(mapper_param_sym_id)
		{
			let mut err = err(
				find,
				"find prop value is not a reference to the map parameter",
			);
			err.labels
				.push((find.span, "find parameter is declared here".into()));
			return Err(err);
		}
		let all = obj
			.parse_bool_flag("all")
			.map_err(|e| err(e, "expected a boolean"))?;
		let no_warn = obj
			.parse_bool_flag("noWarn")
			.map_err(|e| err(e, "expected a boolean"))?;
		let predicate = obj
			.get_property("predicate")
			.map(|p| &p.value)
			.into();
		let replacement = self.parse_replacement(
			&obj.get_property("replacement")
				.ok_or_else(|| {
					err(obj, "patch object missing `replacement` property")
				})?
				.value,
		)?;
		ret.reserve(arr.elements.len());
		for e in &arr.elements {
			let find_expr = e
				.as_expression()
				.ok_or_else(|| err(e, "array element is not an expression"))?;
			let span = find_expr.span();
			let find = self.try_into_raw_match_like(find_expr)?;

			ret.push(RawPatch {
				all,
				no_warn,
				predicate,
				find,
				replacement: OxcVec::from_iter_in(
					replacement.iter().cloned(),
					self,
				),
				span,
			});
		}

		Ok(())
	}

	fn get_arrow_single_return_value(
		func: &'ast ArrowFunctionExpression<'ast>,
	) -> Option<&'ast Expression<'ast>> {
		// TODO: use CFG to get return value of arrow function that might have a body
		if let Some(expr) = func.get_expression() {
			Some(expr)
		} else if let [Statement::ReturnStatement(ret)] = func
			.body
			.as_function_body()
			.expect("we just checked it's not a expr body")
			.statements
			.as_slice()
		{
			ret.argument.as_ref()
		} else {
			None
		}
	}
	// TODO: Cache this
	// maybe noop the replace and just test that the find matches at least once
	fn raw_patches(&self) -> PResult<OxcVec<'ast, RawPatch<'ast>>> {
		let mut ret = OxcVec::new_in(self);
		let define_plugin = self
			.define_plugin()
			.map_err(|e| err_ns("Failed to find definePlugin").s(e.clone()))?;
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
				if let Err(e) = self.parse_spread_patch(spread, &mut ret) {
					let plugin_name = self.plugin_name();
					let inner = miette::Report::from(e);
					let report = LocalSource {
						name: self.path,
						source: self.txt,
						inner,
					};
					warn!(
						"Failed to parse spread patch for plugin {plugin_name:?}, skipping. Cause: \n{report:?}"
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

	fn canonicalize_replace_func(
		&self,
		f: &'ast ArrowFunctionExpression<'ast>,
	) -> PResult<ReplaceLike> {
		let f_ret =
			Self::get_arrow_single_return_value(f).ok_or_else(|| {
				err(
					&f.body,
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
		let mut used_replace_capture_spans =
			vec![Vec::new(); f.params.items.len()];
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
			used_replace_capture_spans[i].push(ident.span());
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
			used_replace_capture_spans[capture_idx as usize]
				.push(ident_ref.span());
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
			used_replace_capture_spans,
		};

		Ok(ret)
	}

	/// Collects every `$self.<prop>` reference in a replacement string, mapping
	/// each referenced property name to the span(s) of the `<prop>` identifier.
	///
	/// `offset` is added to all spans (pass `replace_span.start + 1` to skip the
	/// opening quote), so the returned spans are absolute file offsets pointing
	/// at just the `<prop>` identifier. `$$` is treated as a literal-`$` escape,
	/// so `$$self.x` is not a reference. Only top-level `.`-access is captured
	/// (`$self.a.b` -> `a`); computed access (`$self["x"]`) and bare `$self` are
	/// ignored.
	pub(crate) fn collect_self_references(
		value: &str,
		offset: u32,
	) -> HashMap<SmolStr, Vec<Span>> {
		let bytes = value.as_bytes();
		let mut refs: HashMap<SmolStr, Vec<Span>> = HashMap::new();
		let mut i = 0;
		while i < bytes.len() {
			if bytes[i] != b'$' {
				i += 1;
				continue;
			}
			// `$$` -> literal `$` escape; consume both, not a `$self` ref.
			if bytes.get(i + 1) == Some(&b'$') {
				i += 2;
				continue;
			}
			if bytes[i + 1..].starts_with(b"self.") {
				let prop_start = i + "$self.".len();
				let mut end = prop_start;
				if matches!(bytes.get(end), Some(c) if is_ident_start(*c)) {
					end += 1;
					while matches!(bytes.get(end), Some(c) if is_ident_continue(*c))
					{
						end += 1;
					}
					let prop = &value[prop_start..end];
					let span = Span::new(
						offset + prop_start as u32,
						offset + end as u32,
					);
					refs.entry(prop.into())
						.or_default()
						.push(span);
					i = end;
					continue;
				}
			}
			i += 1;
		}
		refs
	}

	/// offset is added to the returned spans
	///
	/// TODO: named groups?
	fn collect_replace_capture_spans(
		value: &str,
		offset: u32,
	) -> Vec<Vec<Span>> {
		let bts = value.as_bytes();
		let mut it = (0..bts.len()).peekable();
		let mut spans = Vec::new();
		while let Some(i) = it.next() {
			if bts[i] != b'$' {
				continue;
			}
			let Some(n) = it.peek().copied() else {
				continue;
			};
			match bts[n] {
				// literal $ escape
				b'$' => {
					it.next();
				}
				b'&' => {
					if spans.is_empty() {
						spans.push(Vec::new());
					}
					spans[0].push(Span::new(
						i as u32 + offset,
						i as u32 + 2 + offset,
					));
					it.next();
				}
				b'1'..=b'9' => {
					it.next();
					let mut end_idx = n;
					let mut group_num = usize::from(bts[n] - b'0');
					while let Some(idx) = it.peek().copied() {
						if !bts[idx].is_ascii_digit() {
							break;
						}
						group_num *= 10;
						group_num += usize::from(bts[idx] - b'0');
						end_idx += 1;
						it.next();
					}
					if group_num >= spans.len() {
						spans.resize_with(group_num + 1, Vec::new);
					}
					spans[group_num].push(Span::new(
						i as u32 + offset,
						end_idx as u32 + 1 + offset,
					));
				}
				// TODO: named capture groups?
				// b'<' => {
				// 	it.next();
				// 	let mut found_closing = false;
				// 	let mut end_idx = usize::MAX;
				// 	for i in it.by_ref() {
				// 		if bts[i] == b'>' {
				// 			found_closing = true;
				// 			end_idx = i;
				// 			break;
				// 		}
				// 	}
				// 	if found_closing {
				// 		debug_assert_eq!(bts[n], b'<');
				// 		debug_assert_eq!(bts[end_idx], b'>');
				// 		bts[n] = b'{';
				// 		bts[end_idx] = b'}';
				// 	} else {
				// 		// un-terminated, do nothing
				// 		return;
				// 	}
				// }
				_ => {}
			}
		}
		spans
	}

	/// Convert a `RawPatch` into the canonical `Patch` form.
	///
	/// `apply_regress_canon`: when `true`, applies rewrites that make the
	/// patch evaluable by the `regress` crate (`\i` → identifier class in the
	/// regex, `$&` → `$0` / `$<name>` → `${name}` in string replacements). The
	/// LSP sets this to `false` since it ships patches to a JS runtime and
	/// never evaluates them locally; the offline reporter sets it to `true`.
	fn canonicalize_patch(
		&self,
		raw: RawPatch<'ast>,
		apply_regress_canon: bool,
	) -> PResult<Patch> {
		let all = raw.all;
		let no_warn = raw.no_warn;
		let find =
			self.canonicalize_match_like(&raw.find, apply_regress_canon)?;
		let mut replacement = Vec::with_capacity(raw.replacement.len());

		for r in raw.replacement {
			let match_ =
				self.canonicalize_match_like(&r.match_, apply_regress_canon)?;
			let no_warn = r.no_warn;
			let replace = match &r.replace {
				RawReplace::String(StringLiteral { value, span, .. })
				| RawReplace::ComputedString(value, span) => {
					let mut value = value.to_string();
					// span.start is the `"` or `'` so we have to add one
					let used_replace_capture_spans =
						Self::collect_replace_capture_spans(
							&value,
							span.start + 1,
						);
					if apply_regress_canon {
						canonicalize_replace_for_regress(&mut value);
					}
					ReplaceLike {
						v: Replacer::Str(value),
						s: *span,
						used_replace_capture_spans,
					}
				}
				RawReplace::Func(f) => self.canonicalize_replace_func(f)?,
				RawReplace::Template(TemplateLiteral { span, .. }) => {
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
			span: raw.span,
		})
	}
	fn get_finds_(&self) -> PResult<Vec<FindUse>> {
		let mut ret = Vec::new();
		for call in self.get_find_uses() {
			if call.arguments.is_empty() {
				continue;
			}
			let mut args = Vec::with_capacity(call.arguments.len());
			for arg in &call.arguments {
				args.push(Self::parse_find_arg(arg)?);
			}
			let range = call.span;
			ret.push(FindUse {
				range,
				data: FindData {
					kind: self.parse_find_kind(
						call.callee.as_identifier().unwrap(),
					)?,
					args,
				},
			});
		}
		Ok(ret)
	}
	fn parse_find_kind(&self, name: &impl GetSymId) -> PResult<AnyFindType> {
		let name = self.sym_id_of(name).unwrap();
		let decl_source = &self
			.sema
			.symbol_declaration(name)
			.kind()
			.as_import_specifier()
			.unwrap()
			.imported;
		let n = decl_source.name().as_str();
		let n = n.strip_prefix("find").ok_or_else(|| {
			err(decl_source, "Expected find import to start with `find`")
		})?;
		const KINDS: &[(&str, FindType)] = &[
			("Component", FindType::Component),
			("ByProps", FindType::ByProps),
			("Store", FindType::Store),
			("ByCode", FindType::ByCode),
			("ModuleId", FindType::ModuleId),
			("ComponentByCode", FindType::ComponentByCode),
			("CssClasses", FindType::CssClasses),
		];
		let (n, find_type) = 'l: {
			for (prefix, kind) in KINDS {
				if let Some(n) = n.strip_prefix(prefix) {
					break 'l (n, *kind);
				}
			}
			return Err(err(
				decl_source,
				format!("Unknown find type {}", decl_source.name()),
			));
		};
		if !matches!(n.len(), 0 | 4) || (n.len() == 4 && n != "Lazy") {
			return Err(err(
				decl_source,
				"Unexpected suffix on find import, expected either no suffix or `Lazy`",
			));
		}
		let lazy = n.len() == 4;
		Ok(AnyFindType { find_type, lazy })
	}
	fn parse_find_arg(arg: &'ast Argument<'ast>) -> PResult<FindArg> {
		match arg {
			Argument::RegExpLiteral(regex) => {
				let pattern = regex.regex.pattern.text.to_string();
				let flags = regex.regex.flags.to_string();
				Ok(FindArg::Regex { flags, pattern })
			}
			Argument::StringLiteral(string_lit) => {
				Ok(FindArg::String(string_lit.value.to_string()))
			}
			Argument::TemplateLiteral(template_lit)
				if template_lit.is_no_substitution_template() =>
			{
				Ok(FindArg::String(
					template_lit
						.single_quasi()
						.unwrap()
						.to_string(),
				))
			}
			Argument::ArrowFunctionExpression(fn_expr) => {
				Err(err(fn_expr.as_ref(), "TODO: Support function exprs"))
			}
			Argument::FunctionExpression(fn_expr) => {
				Err(err(fn_expr.as_ref(), "TODO: Support function exprs"))
			}
			_ => Err(err(arg, "Unsupported find argument type")),
		}
	}

	fn get_find_uses(&self) -> Vec<&'ast CallExpression<'ast>> {
		let Some(webpack_import) = self.find_import_by_name(FIND_IMPORT_SOURCE)
		else {
			return Vec::new();
		};
		let default_var = OxcVec::new_in(self);
		let import_syms = webpack_import
			.specifiers
			.as_ref()
			.unwrap_or(&default_var)
			.iter()
			.filter_map(|import| {
				if let ImportDeclarationSpecifier::ImportSpecifier(node) =
					import
				{
					Some(node)
				} else {
					None
				}
			})
			.filter(|i| i.imported.name().starts_with("find"))
			.map(|i| i.local.symbol_id());
		let mut calls = Vec::new();
		for sym in import_syms {
			for parent_id in self.refs(sym) {
				let Some(parent_call) = self.p(parent_id).as_call_expression()
				else {
					continue;
				};
				calls.push(parent_call);
			}
		}
		calls
	}

	fn collect_capture_group_spans(
		&self,
		regex: &'ast RegExpLiteral,
	) -> Vec<Span> {
		let groups = collect_capture_groups(
			self,
			regex
				.regex
				.pattern
				.pattern
				.as_ref()
				.unwrap(),
		);
		let mut spans = Vec::with_capacity(groups.indexed_groups.len());
		for cap in groups.indexed_groups {
			spans.push(cap.group.span);
		}
		spans
	}

	/// Convert a parsed `find:` / `match:` value into its canonical form.
	///
	/// `apply_regress_canon` controls regress-specific rewrites — currently the
	/// `\i` → `(?:[A-Za-z_$][\w$]*)` expansion via `canonicalize_regex_ident`.
	/// Pass `true` when the result will be evaluated by the `regress` crate
	/// (e.g. the offline reporter); pass `false` when shipping over the wire to
	/// a JS runtime, which understands either form but doesn't need the
	/// expansion. The Vencord `#{intl::...}` macro is always expanded regardless,
	/// because both sides need the hashed form.
	fn canonicalize_match_like(
		&self,
		raw: &RawMatchLike<'ast>,
		apply_regress_canon: bool,
	) -> PResult<MatchLike> {
		let ret = match raw {
			RawMatchLike::String(StringLiteral { value, span, .. })
			| RawMatchLike::ComputedString(value, span) => {
				let value = canonicalize_intl(value, false, *span)?;
				MatchLike {
					v: Match::Str(Finder::new(value.as_bytes()).into_owned()),
					s: *span,
				}
			}
			RawMatchLike::Regex(pat) => {
				let flags = pat.regex.flags;
				let span = pat.span;
				let capture_spans = self.collect_capture_group_spans(pat);
				let pat = pat.regex.pattern.text.as_str();
				let pat = canonicalize_intl(pat, true, span)?;
				let pat = if apply_regress_canon {
					canonicalize_regex_ident(&pat)
				} else {
					pat
				};
				MatchLike {
					v: Match::Regex(MatchRegex {
						pattern: pat.into_owned(),
						flags,
						regex: None,
						capture_spans,
					}),
					s: span,
				}
			}
			RawMatchLike::Template(TemplateLiteral { span, .. }) => {
				return Err(err(
					span,
					"TODO: Support inlining template literals in match like",
				));
			}
		};

		Ok(ret)
	}
}

/// Whether `b` is valid as the first byte of a JS identifier (ASCII subset,
/// sufficient for plugin top-level keys).
const fn is_ident_start(b: u8) -> bool {
	b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

/// Whether `b` is valid as a non-leading byte of a JS identifier (ASCII subset).
const fn is_ident_continue(b: u8) -> bool {
	b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
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

impl<'ast> GetAllocator<'ast> for VencordAstParser<'ast> {
	fn allocator(&self) -> &'ast Allocator {
		self.alloc
	}
}

impl<'ast> GetAstBuilder<'ast> for VencordAstParser<'ast> {
	type Builder = AstBuilder<'ast>;

	fn builder(&self) -> &Self::Builder {
		&self.ast_builder
	}
}

impl<'ast> ConstantEvaluationCtx<'ast> for VencordAstParser<'ast> {}

impl<'ast> AstParser<'ast> for VencordAstParser<'ast> {
	fn prog(&self) -> &'ast Program<'ast> {
		self.prog
	}

	fn sema(&self) -> &Semantic<'ast> {
		&self.sema
	}

	fn node_location_index(&self) -> &cache::Ref<NodeLocationIndex<'ast>> {
		&self.cache.node_index
	}
}

impl<'ast> ESModuleParser<'ast> for VencordAstParser<'ast> {}
