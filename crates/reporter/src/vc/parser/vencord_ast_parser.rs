use crate::{
    util::Stage,
    vc::{
        Match, MatchLike, MatchRegex, Patch, Plugin, ReplaceLike, Replacement, Replacer,
        hash::hash_message_key,
        parser::{
            ast_parser::{AstParser, ESModuleParser, ParsedAst},
            exts::{
                ArrayExpressionElementExt as _, BindingPatternExt, ExpressionExt,
                ImportDeclarationExt, ModuleDeclarationExt, ObjectExpressionExt,
               
            },
            patches::{RawMatchLike, RawPatch, RawReplace, RawReplacement, canonicalize_patch},
        },
    },
};
use anyhow::{Context, Result, bail};
use itertools::Itertools;
use memchr::memmem::Finder;
use oxc::{
    allocator::{Allocator, Box, Vec as OxcVec},
    ast::{
        AstBuilder, AstKind,
        ast::{
            Argument, ArrayExpressionElement, ArrowFunctionExpression, Expression,
            ImportDeclaration, ModuleDeclaration, ObjectExpression, RegExpLiteral, SpreadElement,
            StringLiteral, TemplateLiteral,
        },
    },
    cfg::{BlockNodeId, ControlFlowGraph},
    minifier::PropertyReadSideEffects,
    parser::{Parser as OxcParser, ParserReturn},
    semantic::{
        AstNode, NodeId, Semantic, SemanticBuilder,
        dot::{DebugDot, DebugDotContext},
    },
    span::{Atom, SourceType, Span},
};
use oxc_ecmascript::{
    GlobalContext,
    constant_evaluation::{ConstantEvaluation, ConstantEvaluationCtx},
    side_effects::MayHaveSideEffectsContext,
};
use regress::Regex;
use std::{borrow::Cow, cell::Cell, fmt::Debug, sync::LazyLock};
use tracing::{debug, warn};

pub struct VencordAstParser<'ast> {
    alloc: &'ast Allocator,
    ast_builder: AstBuilder<'ast>,
    ast: &'ast ParsedAst<'ast>,
    sema: Semantic<'ast>,
}

const DEFINE_PLUGIN_IMPORT_SOURCE: &str = "@utils/types";

impl<'ast> VencordAstParser<'ast> {
    pub fn try_new(alloc: &'ast Allocator, source: &'ast str) -> Result<Self> {
        let (ast, sema) = Self::parse(alloc, source, SourceType::tsx())?;
        Ok(Self {
            alloc,
            ast_builder: AstBuilder::new(alloc),
            ast,
            sema,
        })
    }

    fn define_plugin<'a: 'ast>(&'a self) -> Option<&'ast ObjectExpression<'ast>> {
        let define_plugin = self
            .find_import_by_name(DEFINE_PLUGIN_IMPORT_SOURCE)?
            .default_var()?;
        for var_use in self.sema.symbol_references(define_plugin) {
            let AstKind::CallExpression(call) = self.p(var_use.node_id()).kind() else {
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
            Expression::StringLiteral(s) => Ok(RawMatchLike::String(s.as_ref())),
            Expression::TemplateLiteral(t) => Ok(RawMatchLike::Template(t.as_ref())),
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
        let predicate = obj.get_property("predicate").map(|p| &p.value).into();
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
            Expression::ArrowFunctionExpression(s) => Ok(RawReplace::Func(s.as_ref())),
            Expression::TemplateLiteral(s) => Ok(RawReplace::Template(s.as_ref())),
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
                bail!("Spreads and dynamic expressions are not supported in patches yet.")
            }
            ArrayExpressionElement::ObjectExpression(obj) => obj.as_ref(),
            _ => {
                bail!("invalid element in patches array, got: {}", obj.dbg_name());
            }
        };

        let all = obj.parse_bool_flag("all")?;
        let no_warn = obj.parse_bool_flag("noWarn")?;
        let predicate = obj.get_property("predicate").map(|p| &p.value).into();
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
                let mut ret = OxcVec::with_capacity_in(elements.len(), self.alloc);
                for elem in elements {
                    let elem = elem
                        .as_expression()
                        .and_then(Expression::as_object_expression)
                        .context("invalid replacement type")?;
                    ret.push(self.try_into_raw_replacement(elem)?);
                }
                ret
            }
            Expression::ObjectExpression(obj) => {
                OxcVec::from_array_in([self.try_into_raw_replacement(obj.as_ref())?], self.alloc)
            }
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
        if mapper.params.parameters_count() != 1 || mapper.params.rest.is_some() {
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
        let arr = map_prop.object().as_array_expression()?;
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

        if self.sema.scoping().get_reference(find).symbol_id() != Some(mapper_param) {
            return None;
        }
        let all = obj.parse_bool_flag("all").ok()?;
        let no_warn = obj.parse_bool_flag("noWarn").ok()?;
        let predicate = obj.get_property("predicate").map(|p| &p.value).into();
        let replacement = self
            .parse_replacement(&obj.get_property("replacement")?.value)
            .ok()?;
        ret.reserve(arr.elements.len());
        for e in &arr.elements {
            let find = e.as_expression()?;
            let find = self.try_into_raw_match_like(find).ok()?;

            ret.push(RawPatch {
                all,
                no_warn,
                predicate,
                find,
                replacement: OxcVec::from_iter_in(replacement.iter().cloned(), self.alloc),
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

    fn raw_patches<'a: 'ast>(&'a self) -> Result<OxcVec<'ast, RawPatch<'ast>>> {
        let mut ret = OxcVec::new_in(self.alloc);
        let Some(patches) = self
            .define_plugin()
            .and_then(|o| Some(&o.get_property("patches")?.value))
        else {
            debug!("No patches found for plugin");
            return Ok(ret);
        };

        let Expression::ArrayExpression(patches) = patches else {
            bail!("invalid type for patches, expected array");
        };

        let patches = patches.as_ref();

        for patch_obj in &patches.elements {
            if let Some(spread) = patch_obj.as_spread() {
                if self.parse_spread_patch(spread, &mut ret).is_none() {
                    let plugin_name = self.plugin_name();
                    warn!("Failed to parse spread patch for plugin {plugin_name:?}, skipping");
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

        debug!(
            "Parsed {} patches for plugin {:?}",
            ret.len(),
            self.plugin_name()
        );

        Ok(ret)
    }

    pub fn patches(&self) -> Result<Vec<Patch>> {
        let name = self.plugin_name().unwrap_or("<unknown plugin>");
        let ret = self
            .raw_patches()?
            .into_iter()
            .filter_map(|raw| match canonicalize_patch(raw) {
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
    fn is_global_reference(&self, _reference: &oxc::ast::ast::IdentifierReference<'_>) -> bool {
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
    fn ast(&self) -> &'ast ParsedAst<'ast> {
        self.ast
    }

    fn sema(&self) -> &Semantic<'ast> {
        &self.sema
    }
}

impl<'ast> ESModuleParser<'ast> for VencordAstParser<'ast> {}
