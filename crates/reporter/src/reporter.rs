use std::{collections::HashSet, sync::Arc};

use crate::vc::{Match, Patch, Plugin};
use anyhow::{Result, anyhow};
use explorer_types::FullBundle;
use miette::{Diagnostic, NamedSource, Report, SourceSpan};
use oxc::{
    allocator::Allocator,
    ast::ast::Program,
    codegen::{Codegen, CodegenOptions, CommentOptions, IndentChar, LegalComment},
    diagnostics::OxcDiagnostic,
    parser::Parser,
    semantic::{SemanticBuilder, Stats},
    span::{SourceType, Span},
};
use regress::{Regex, escape};
use thiserror::Error;
use tokio::{sync::mpsc, task};
use tracing::{debug, warn};

#[derive(Debug)]
pub enum Msg {
    Progress,
    Error(ReporterError),
    Done(Result<()>),
}

impl From<ReporterError> for Msg {
    fn from(v: ReporterError) -> Self {
        Self::Error(v)
    }
}

#[derive(Error, Debug, Diagnostic)]
pub enum ReporterError {
    #[error("Bad Regex Syntax")]
    #[diagnostic[
        code(reporter::bad_regex_syntax),
    ]]
    BadRegexSyntax {
        plugin_id: u16,
        #[source]
        source: anyhow::Error,
        #[label("From this regex")]
        regex_span: SourceSpan,
    },
    #[error("Replace Match Not Found")]
    #[diagnostic[
        code(reporter::replace::match_not_found),
        help("This error occurred in module {module_id}")
    ]]
    ReplaceMatchNotFound {
        #[label("Caused by this match")]
        match_span: SourceSpan,
        module_id: u32,
        plugin_id: u16,
    },
    #[error("Replace Match Ambiguous")]
    #[diagnostic[
        code(reporter::replace::match_ambiguous),
        help("This error occurred in module {module_id}")        
    ]]
    ReplaceMatchAmbiguous {
        #[label("Caused by this match")]
        match_span: SourceSpan,
        plugin_id: u16,
        module_id: u32,
    },

    #[error("Replace Syntax Error")]
    #[diagnostic[
        code(reporter::replace::syntax_error),
        help("This error occurred in module {module_id}"),
    ]]
    ReplaceSyntaxError {
        #[label("Caused by this replacement")]
        replace_span: SourceSpan,
        #[source]
        #[diagnostic_source]
        cause: Box<dyn Diagnostic + Send + Sync>,
        module_id: u32,
        plugin_id: u16,
    },
    #[error("Find Ambiguous")]
    #[diagnostic[
        code(reporter::find_ambiguous),
        help("This error occurred in module {module_id}"),
    ]]
    FindAmbiguous {
        #[label("Caused by this find")]
        find_span: SourceSpan,
        plugin_id: u16,
        module_id: u32,
    },
}

impl ReporterError {
    pub const fn plugin_id(&self) -> u16 {
        match self {
            Self::BadRegexSyntax { plugin_id, .. }
            | Self::ReplaceMatchNotFound { plugin_id, .. }
            | Self::ReplaceMatchAmbiguous { plugin_id, .. }
            | Self::ReplaceSyntaxError { plugin_id, .. }
            | Self::FindAmbiguous { plugin_id, .. } => *plugin_id,
        }
    }
}

#[track_caller]
pub fn report_broken_patches(
    target_build: Arc<FullBundle>,
    plugins: Arc<Vec<Plugin>>,
) -> mpsc::Receiver<Msg> {
    let (mut tx, rx) = mpsc::channel(1024);
    task::spawn_blocking(move || {
        run_reporter(&target_build, &plugins, &mut tx);
        tx.blocking_send(Msg::Done(Ok(()))).unwrap();
    });

    rx
}

#[derive(Debug, Copy, Clone)]
enum PendingAction<'a> {
    MissingToGood(&'a Patch),
    MissingToBad(&'a Patch),
    GoodToBad(&'a Patch),
}

fn run_reporter(build: &FullBundle, plugins: &[Plugin], tx: &mut mpsc::Sender<Msg>) -> () {
    let mut missing: HashSet<&Patch> = plugins.iter().flat_map(|p| p.patches.iter()).collect();
    let mut good: HashSet<&Patch> = HashSet::with_capacity(missing.len());
    let mut bad: HashSet<&Patch> = HashSet::new();
    let mut pending: Vec<PendingAction> = Vec::new();
    let mut alloc = Allocator::new();
    for (m_id, m_txt) in &build.modules {
        alloc.reset();
        // stats about the ast for m_txt
        // used by oxc to optimize allocations on re-parse
        let mut stats = None;
        let m_filename = format!("{m_id}.js");
        for &patch in &missing {
            let plugin_id = patch.plugin_id();
            if let Some(a) = matches_module(Some(tx), m_txt, patch, plugin_id) {
                if !a {
                    continue;
                }
            } else {
                pending.push(PendingAction::MissingToBad(patch));
                continue;
            }
            let mut last_src = format!("0,{m_txt}");
            let mut action = PendingAction::MissingToGood(patch);
            for r in &patch.replacement {
                let pat = match &r.match_.v {
                    Match::Str(s) => {
                        debug!("Implicit converting string replacement match to regex");
                        // we are creating a regex of an escaped string
                        // this should be impossible to fail
                        &Regex::new(&escape(s.as_str())).unwrap()
                    }
                    Match::Regex(v) => match v.regex() {
                        Ok(r) => r,
                        Err(e) => {
                            tx.blocking_send(
                                ReporterError::BadRegexSyntax {
                                    plugin_id,
                                    source: anyhow!("{e:?}"),
                                    regex_span: r.match_.s.into(),
                                }
                                .into(),
                            )
                            .unwrap();
                            action = PendingAction::MissingToBad(patch);
                            continue;
                        }
                    },
                };
                let mut it = pat.find_iter(&last_src);
                // TODO: handle group patches
                if it.next().is_none() {
                    tx.blocking_send(
                        ReporterError::ReplaceMatchNotFound {
                            module_id: *m_id,
                            plugin_id,
                            match_span: r.match_.s.into(),
                        }
                        .into(),
                    )
                    .unwrap();
                    action = PendingAction::MissingToBad(patch);
                    continue;
                }

                if it.next().is_some() {
                    tx.blocking_send(
                        ReporterError::ReplaceMatchAmbiguous {
                            module_id: *m_id,
                            plugin_id,
                            match_span: r.match_.s.into(),
                        }
                        .into(),
                    )
                    .unwrap();
                }

                let new_src = match &r.replace.v {
                    crate::vc::Replacer::Str(s) => pat.replace(&last_src, s.as_str()),
                };

                let chk = {
                    let chk = check_syntax_errors(&alloc, &new_src, stats, Some(&m_filename));
                    alloc.reset();
                    chk
                };
                match chk {
                    Ok(s) => stats = Some(s),
                    Err(e) => {
                        tx.blocking_send(
                            ReporterError::ReplaceSyntaxError {
                                module_id: *m_id,
                                plugin_id,
                                replace_span: r.replace.s.into(),
                                cause: e,
                            }
                            .into(),
                        )
                        .unwrap();
                        action = PendingAction::MissingToBad(patch);
                        continue;
                    }
                }
                last_src = new_src;
            }
            pending.push(action);
        }
        for &patch in &good {
            if patch.all {
                continue;
            }
            // matches_module returns None if there was an error.
            // It would be a logic error for a bad regex to be in the good set
            if matches_module(None, m_txt, patch, patch.plugin_id()).unwrap() {
                tx.blocking_send(
                    ReporterError::FindAmbiguous {
                        module_id: *m_id,
                        plugin_id: patch.plugin_id(),
                        find_span: patch.find.s.into(),
                    }
                    .into(),
                )
                .unwrap();
                pending.push(PendingAction::GoodToBad(patch));
            }
        }
        for &patch in &bad {
            if patch.all {
                continue;
            }

            if matches_module(None, m_txt, patch, patch.plugin_id()) == Some(true) {
                tx.blocking_send(
                    ReporterError::FindAmbiguous {
                        module_id: *m_id,
                        plugin_id: patch.plugin_id(),
                        find_span: patch.find.s.into(),
                    }
                    .into(),
                )
                .unwrap();
            }
        }
        for action in &pending {
            match action {
                PendingAction::GoodToBad(p) => {
                    if !p.all {
                        debug_assert!(
                            good.contains(p),
                            "GoodToBad action on patch that is not in good set"
                        );
                        debug_assert!(
                            !bad.contains(p),
                            "GoodToBad action on patch that is already in bad set"
                        );
                    }
                    good.remove(p);
                    bad.insert(p);
                }
                PendingAction::MissingToGood(p) => {
                    if !p.all {
                        debug_assert!(
                            missing.contains(p),
                            "MissingToGood action on patch that is not in missing set"
                        );
                        debug_assert!(
                            !good.contains(p),
                            "MissingToGood action on patch that is already in good set"
                        );
                        missing.remove(p);
                    }
                    good.insert(p);
                }
                PendingAction::MissingToBad(p) => {
                    if !p.all {
                        debug_assert!(
                            missing.contains(p),
                            "MissingToBad action on patch that is not in missing set"
                        );
                        debug_assert!(
                            !bad.contains(p),
                            "MissingToBad action on patch that is already in bad set"
                        );
                        missing.remove(p);
                    }
                    bad.insert(p);
                }
            }
        }
        pending.clear();
        match tx.try_send(Msg::Progress) {
            Ok(()) => {}
            Err(e) => {
                warn!("Failed to send progress update: {e}");
            }
        }
    }
}

fn matches_module(
    tx: Option<&mut mpsc::Sender<Msg>>,
    m_txt: &str,
    patch: &Patch,
    plugin_id: u16,
) -> Option<bool> {
    Some(match &patch.find.v {
        crate::vc::Match::Str(s) => m_txt.contains(s),
        crate::vc::Match::Regex(s) => {
            let s = match s.regex() {
                Ok(r) => r,
                Err(e) => {
                    if let Some(tx) = tx {
                        tx.blocking_send(
                            ReporterError::BadRegexSyntax {
                                source: anyhow!("{e:?}"),
                                plugin_id,
                                regex_span: patch.find.s.into(),
                            }
                            .into(),
                        )
                        .unwrap();
                    }
                    return None;
                }
            };
            s.find(m_txt).is_some()
        }
    })
}

fn check_syntax_errors(
    alloc: &Allocator,
    src: &str,
    stats: Option<Stats>,
    filename: Option<&str>,
) -> Result<Stats, Box<dyn Diagnostic + Send + Sync>> {
    let mut p_ret = Parser::new(alloc, src, SourceType::unambiguous()).parse();
    if !p_ret.errors.is_empty() {
        let ret = p_ret.errors.swap_remove(0);
        return Err(ret.into());
    }
    let sema = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .with_cfg(false);
    let sema = if let Some(stats) = stats {
        sema.with_stats(stats)
    } else {
        sema
    };
    let mut sema = sema.build(&p_ret.program);
    if sema.errors.is_empty() {
        Ok(sema.semantic.stats())
    } else {
        let ret = sema.errors.swap_remove(0);
        Err(ret.into())
    }
}

fn pretty_print(program: &Program<'_>) -> String {
    Codegen::new()
        .with_options(CodegenOptions {
            indent_char: IndentChar::Tab,
            indent_width: 1,
            initial_indent: 0,
            minify: false,
            single_quote: false,
            source_map_path: None,
            comments: CommentOptions {
                annotation: true,
                jsdoc: true,
                legal: LegalComment::Inline,
                normal: true,
            },
        })
        .build(program)
        .code
}
