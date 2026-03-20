mod exts;
use std::{borrow::Borrow as _, cell::Cell, fmt::Debug, fs, path::Path};

use crate::vc::{
    Patch,
    parser::exts::{
        ExpressionExt, ImportDeclarationExt, ModuleDeclarationExt, ObjectExpressionExt,
    },
};
use anyhow::{Context, Result, bail};
use oxc::{
    allocator::{Allocator, Box, Vec as OxcVec},
    ast::{
        AstKind,
        ast::{
            Argument, ArrayExpression, ArrayExpressionElement, ArrowFunctionExpression, Expression,
            ImportDeclaration, ModuleDeclaration, ObjectExpression, ObjectProperty, RegExpLiteral,
            StringLiteral, TemplateLiteral,
        },
    },
    parser::{Parser as OxcParser, ParserReturn},
    semantic::{AstNode, NodeId, Semantic, SemanticBuilder},
    span::SourceType,
};
use thiserror::Error;
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
        sema,
        c: ParserCache::default(),
    };

    info!("plugin name: {:?}", parser.plugin_name());

    info!("Parsing patches...");

    let p = parser.raw_patches().with_context(|| {
        format!(
            "Failed to parse patches for plugin {:?}",
            parser.plugin_name()
        )
    })?;

    dbg!(p);

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
        .build(&ast.program);

    if !sema_result.errors.is_empty() {
        Err(sema_result.errors.swap_remove(0)).context("Failed to validate plugin AST")?;
    }

    Ok(sema_result.semantic)
}

pub struct Parser<'ast> {
    alloc: &'ast Allocator,
    ast: &'ast ParserReturn<'ast>,
    sema: Semantic<'ast>,
    c: ParserCache<'ast>,
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

#[derive(Debug)]
struct RawReplacement<'ast> {
    match_: RawMatchLike<'ast>,
    replace: RawReplace<'ast>,
    no_warn: bool,
    predicate: PatchPredicate<'ast>,
    // i don't think vencord uses these at all
    // from_build: Option<u32>,
    // to_build: Option<u32>,
}

impl<'a> TryFrom<&'a ObjectExpression<'a>> for RawReplacement<'a> {
    type Error = anyhow::Error;

    fn try_from(obj: &'a ObjectExpression<'a>) -> std::result::Result<Self, Self::Error> {
        let match_ = obj
            .get_property("match")
            .context("replacement missing match")?
            .value
            .borrow()
            .try_into()?;
        let replace = obj
            .get_property("replace")
            .context("replacement missing replace")?
            .value
            .borrow()
            .try_into()?;
        let no_warn = obj.parse_bool_flag("noWarn")?;
        let predicate = obj.get_property("predicate").map(|p| &p.value).into();
        Ok(RawReplacement {
            match_,
            replace,
            no_warn,
            predicate,
        })
    }
}

enum RawReplace<'ast> {
    String(&'ast StringLiteral<'ast>),
    // TODO: further parse this
    Func(&'ast ArrowFunctionExpression<'ast>),
}

#[automatically_derived]
impl Debug for RawReplace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => f.debug_tuple("String").field(&s.value).finish(),
            Self::Func(_) => f.debug_tuple("Func").field(&"...").finish(),
        }
    }
}

impl<'a> TryFrom<&'a Expression<'a>> for RawReplace<'a> {
    type Error = anyhow::Error;

    fn try_from(value: &'a Expression<'a>) -> std::result::Result<Self, Self::Error> {
        match value {
            Expression::StringLiteral(s) => Ok(Self::String(s.as_ref())),
            Expression::ArrowFunctionExpression(s) => Ok(Self::Func(s.as_ref())),
            _ => bail!("invalid replace type"),
        }
    }
}

enum RawMatchLike<'ast> {
    String(&'ast StringLiteral<'ast>),
    Regex(&'ast RegExpLiteral<'ast>),
    Template(&'ast TemplateLiteral<'ast>),
}

#[automatically_derived]
impl Debug for RawMatchLike<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => f.debug_tuple("String").field(&s.value).finish(),
            Self::Regex(r) => f.debug_tuple("Regex").field(&r.regex.pattern.text).finish(),
            Self::Template(t) => f.debug_tuple("Template").field(t).finish(),
        }
    }
}

impl<'a> TryFrom<&'a Expression<'a>> for RawMatchLike<'a> {
    type Error = anyhow::Error;

    fn try_from(value: &'a Expression<'a>) -> Result<Self> {
        match value {
            Expression::RegExpLiteral(r) => Ok(Self::Regex(r.as_ref())),
            Expression::StringLiteral(s) => Ok(Self::String(s.as_ref())),
            Expression::TemplateLiteral(t) => Ok(Self::Template(t.as_ref())),
            _ => bail!("invalid match-like type"),
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

    fn parse_single_patch<'a: 'ast>(
        &'a self,
        obj: &'ast ArrayExpressionElement<'ast>,
    ) -> Result<RawPatch<'ast>> {
        let obj = match obj {
            ArrayExpressionElement::CallExpression(_)
            | ArrayExpressionElement::SpreadElement(_) => {
                bail!("Spreads and dynamic expressions are not supported in patches yet.")
            }
            ArrayExpressionElement::ObjectExpression(obj) => obj.as_ref(),
            _ => {
                bail!("invalid element in patches array, got: {obj:?}");
            }
        };

        let all = obj.parse_bool_flag("all")?;
        let no_warn = obj.parse_bool_flag("noWarn")?;
        let predicate = obj.get_property("predicate").map(|p| &p.value).into();
        let find = obj
            .get_property("find")
            .context("patch missing find")?
            .value
            .borrow()
            .try_into()?;

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
                    ret.push(elem.try_into()?);
                }
                ret
            }
            Expression::ObjectExpression(obj) => {
                OxcVec::from_array_in([obj.as_ref().try_into()?], self.alloc)
            }
            _ => bail!("invalid replacement type"),
        };
        Ok(ret)
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
            let patch = self
                .parse_single_patch(patch_obj)
                .context("Failed to parse patch")?;
            ret.push(patch);
        }

        info!(
            "Parsed {} patches for plugin {:?}",
            ret.len(),
            self.plugin_name()
        );

        Ok(ret)
    }
}
