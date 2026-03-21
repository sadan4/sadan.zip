mod exts;
use std::{
    borrow::{Borrow as _, Cow},
    cell::Cell,
    fmt::Debug,
    fs,
    path::Path,
};

use crate::vc::{
    Patch,
    parser::exts::{
        ArrayExpressionElementExt as _, BindingPatternExt, ExpressionExt, ImportDeclarationExt,
        ModuleDeclarationExt, ObjectExpressionExt, TemplateLiteralExt,
    },
};
use anyhow::{Context, Result, bail};
use oxc::{
    allocator::{Allocator, Box, Vec as OxcVec},
    ast::{
        AstBuilder, AstKind,
        ast::{
            Argument, ArrayExpressionElement, ArrowFunctionExpression, BinaryExpression,
            BinaryOperator, Expression, ImportDeclaration, ModuleDeclaration, ObjectExpression,
            RegExpLiteral, SpreadElement, StringLiteral, TemplateLiteral,
        },
    },
    cfg::{BlockNodeId, ControlFlowGraph},
    minifier::PropertyReadSideEffects,
    parser::{Parser as OxcParser, ParserReturn},
    semantic::{
        AstNode, NodeId, Semantic, SemanticBuilder,
        dot::{DebugDot, DebugDotContext},
    },
    span::{SourceType, Span},
};
use oxc_ecmascript::{
    GlobalContext,
    constant_evaluation::{ConstantEvaluation, ConstantEvaluationCtx},
    side_effects::MayHaveSideEffectsContext,
};
use tracing::{debug, info, warn};

pub fn parse_patches(allocator: &Allocator, plugin_entry: &Path) -> Result<Vec<Patch>> {
    let content = fs::read_to_string(plugin_entry)?;
    let source_type = SourceType::from_path(plugin_entry)
        .context("Failed to parse source type for plugin entry")?;

    let ast = OxcParser::new(allocator, &content, source_type).parse();
    let sema = verify_and_make_sema(&ast)?;

    let parser = Parser {
        alloc: allocator,
        ast: &ast,
        ast_builder: AstBuilder::new(allocator),
        sema,
        c: ParserCache::default(),
    };

    let name = parser
        .plugin_name()
        .unwrap_or_else(|| "<unknown>".to_string());

    debug!("Parsing patches for {name}");

    let _ = parser
        .raw_patches()
        .with_context(|| format!("Failed to parse patches for plugin {name:?}"))?;

    Ok(vec![])
}

fn verify_and_make_sema<'a>(ast: &'a ParserReturn<'a>) -> Result<Semantic<'a>> {
    if !ast.errors.is_empty() {
        // Do we need to clone here?
        Err(ast.errors[0].clone()).context("Failed to parse plugin")?;
    }
    if ast.panicked {
        bail!("Parser panicked while parsing plugin");
    }
    // run semantic analysis
    let mut sema_result = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .with_cfg(true)
        .build(&ast.program);

    if !sema_result.errors.is_empty() {
        Err(sema_result.errors.swap_remove(0)).context("Failed to validate plugin AST")?;
    }

    Ok(sema_result.semantic)
}

pub struct Parser<'ast> {
    alloc: &'ast Allocator,
    ast_builder: AstBuilder<'ast>,
    ast: &'ast ParserReturn<'ast>,
    sema: Semantic<'ast>,
    c: ParserCache<'ast>,
}

impl GlobalContext<'_> for Parser<'_> {
    fn is_global_reference(&self, _reference: &oxc::ast::ast::IdentifierReference<'_>) -> bool {
        false
    }
}

impl MayHaveSideEffectsContext<'_> for Parser<'_> {
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

impl<'ast> ConstantEvaluationCtx<'ast> for Parser<'ast> {
    fn ast(&self) -> AstBuilder<'ast> {
        self.ast_builder
    }
}

#[derive(Default)]
struct ParserCache<'ast> {
    // FIXME: use custom enum instead of Option<Option<>> to be more clear
    define_plugin: Cell<Option<Option<&'ast ObjectExpression<'ast>>>>,
}

#[derive(Debug)]
struct RawPatch<'ast> {
    all: bool,
    no_warn: bool,
    predicate: PatchPredicate<'ast>,
    find: RawMatchLike<'ast>,
    replacement: OxcVec<'ast, RawReplacement<'ast>>,
    // i don't think vencord uses these at all
    // from_build: Option<u32>,
    // to_build: Option<u32>,
}

// TODO: further parse this
#[derive(Copy, Clone)]
struct PatchPredicate<'ast>(Option<&'ast Expression<'ast>>);

impl<'ast> From<Option<&'ast Expression<'ast>>> for PatchPredicate<'ast> {
    fn from(value: Option<&'ast Expression<'ast>>) -> Self {
        Self(value)
    }
}

impl<'ast> Debug for PatchPredicate<'ast> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PatchPredicate")
            .field(&self.0.map(Expression::dbg_name))
            .finish()
    }
}

#[derive(Debug, Clone)]
struct RawReplacement<'ast> {
    match_: RawMatchLike<'ast>,
    replace: RawReplace<'ast>,
    no_warn: bool,
    predicate: PatchPredicate<'ast>,
    // i don't think vencord uses these at all
    // from_build: Option<u32>,
    // to_build: Option<u32>,
}

#[derive(Copy, Clone)]
enum RawReplace<'ast> {
    String(&'ast StringLiteral<'ast>),
    Func(&'ast ArrowFunctionExpression<'ast>),
    Template(&'ast TemplateLiteral<'ast>),
    ComputedString(&'ast str, Span),
}

#[automatically_derived]
impl Debug for RawReplace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => f.debug_tuple("String").field(&s.value).finish(),
            Self::Func(_) => f.debug_tuple("Func").field(&"...").finish(),
            Self::Template(t) => f.debug_tuple("Template").field(&t.dbg_str()).finish(),
            Self::ComputedString(s, _) => f.debug_tuple("ComputedString").field(s).finish(),
        }
    }
}

#[derive(Clone)]
enum RawMatchLike<'ast> {
    String(&'ast StringLiteral<'ast>),
    Regex(&'ast RegExpLiteral<'ast>),
    Template(&'ast TemplateLiteral<'ast>),
    ComputedString(&'ast str, Span),
}

#[automatically_derived]
impl Debug for RawMatchLike<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => f.debug_tuple("String").field(&s.value).finish(),
            Self::Regex(r) => f.debug_tuple("Regex").field(&r.regex.pattern.text).finish(),
            Self::Template(t) => f.debug_tuple("Template").field(t).finish(),
            Self::ComputedString(s, _) => f.debug_tuple("ComputedString").field(s).finish(),
        }
    }
}

const DEFINE_PLUGIN_IMPORT_SOURCE: &str = "@utils/types";

impl<'ast> Parser<'ast> {
    /// node from id
    pub fn n<'a: 'ast>(&'a self, node_id: NodeId) -> &'ast AstNode<'ast> {
        self.sema.nodes().get_node(node_id)
    }
    /// Parent of node
    pub fn p<'a: 'ast>(&'a self, node_id: NodeId) -> &'ast AstNode<'ast> {
        self.sema.nodes().parent_node(node_id)
    }
    pub fn cfg_id(&self, node_id: NodeId) -> BlockNodeId {
        self.sema.nodes().cfg_id(node_id)
    }
    pub fn cfg<'a: 'ast>(&'a self) -> &'ast ControlFlowGraph {
        // we always parse with the cfg
        self.sema.cfg().unwrap()
    }
    pub fn dbg_cfg<'a: 'ast>(&'a self, node_id: NodeId) -> String {
        let cfg_id = self.cfg_id(node_id);
        let cfg = self.cfg();
        let block = cfg.basic_block(cfg_id);
        let ctx = DebugDotContext::new(self.sema.nodes(), true);
        block.debug_dot(ctx)
    }
    pub fn import_statements<'a: 'ast>(
        &'a self,
    ) -> impl Iterator<Item = &'ast ImportDeclaration<'ast>> {
        self.ast.program.body.iter().filter_map(|node| {
            node.as_module_declaration()
                .and_then(ModuleDeclaration::as_import_declaration)
                .map(Box::as_ref)
        })
    }
    pub fn find_import_by_name<'a: 'ast, 'b>(
        &'a self,
        from: &'b str,
    ) -> Option<&'ast ImportDeclaration<'ast>> {
        let pred = |import: &&ImportDeclaration| import.source.value == from;
        debug_assert!(
            self.import_statements().filter(pred).count() <= 1,
            "Found multiple import statements with the same source"
        );
        // Imports can only be at the top level
        self.import_statements().find(pred)
    }
    pub fn define_plugin_<'a: 'ast>(&'a self) -> Option<&'ast ObjectExpression<'ast>> {
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
    pub fn define_plugin<'a: 'ast>(&'a self) -> Option<&'ast ObjectExpression<'ast>> {
        if let Some(s) = self.c.define_plugin.get() {
            return s;
        }
        let ret = self.define_plugin_();

        self.c.define_plugin.set(Some(ret));

        ret
    }
    pub fn plugin_name<'a: 'ast>(&'a self) -> Option<String> {
        Some(
            self.define_plugin()?
                .get_property("name")?
                .value
                .as_string_literal()?
                .value
                .to_string(),
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
                        self.ast_builder.atom_from_cow(&cow).as_str(),
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
                        self.ast_builder.atom_from_cow(&cow).as_str(),
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

    pub fn raw_patches<'a: 'ast>(&'a self) -> Result<OxcVec<'ast, RawPatch<'ast>>> {
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
                    warn!(
                        "Failed to parse spread patch for plugin {:?}, skipping",
                        self.plugin_name()
                    );
                }
            } else {
                match self.parse_single_patch(patch_obj) {
                    Ok(patch) => {
                        ret.push(patch);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse patch for plugin {:?}, skipping. Error: {e:#?}",
                            self.plugin_name(),
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
}
