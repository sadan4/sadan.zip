use std::{collections::HashSet, sync::Arc};

use crate::vc::{Match, Patch, Plugin, Replacer};
use anyhow::{Result, anyhow};
use derive_more::{IsVariant, TryUnwrap};
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

#[derive(Error, Debug, Diagnostic, IsVariant)]
pub enum ReporterError {
    #[error("Bad Regex Syntax")]
    #[diagnostic[
        code(reporter::bad_regex_syntax),
        help("The regex was expanded to {expanded}"),
    ]]
    BadRegexSyntax {
        plugin_id: u16,
        #[source]
        source: anyhow::Error,
        #[label("From this regex")]
        regex_span: SourceSpan,
        expanded: String,
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
        severity(Warning),
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
        severity(Warning),
        help("This error occurred in module {module_id}"),
    ]]
    FindAmbiguous {
        #[label("Caused by this find")]
        find_span: SourceSpan,
        plugin_id: u16,
        module_id: u32,
    },
    #[error(transparent)]
    NoWarn(Box<ReporterError>),
}

impl ReporterError {
    pub const fn plugin_id(&self) -> u16 {
        match self {
            Self::BadRegexSyntax { plugin_id, .. }
            | Self::ReplaceMatchNotFound { plugin_id, .. }
            | Self::ReplaceMatchAmbiguous { plugin_id, .. }
            | Self::ReplaceSyntaxError { plugin_id, .. }
            | Self::FindAmbiguous { plugin_id, .. } => *plugin_id,
            Self::NoWarn(e) => e.plugin_id(),
        }
    }

    pub const fn module_id(&self) -> Option<u32> {
        match self {
            Self::BadRegexSyntax { .. } => None,
            Self::ReplaceSyntaxError { module_id, .. }
            | Self::FindAmbiguous { module_id, .. }
            | Self::ReplaceMatchAmbiguous { module_id, .. }
            | Self::ReplaceMatchNotFound { module_id, .. } => Some(*module_id),
            Self::NoWarn(e) => e.module_id(),
        }
    }

    pub const fn as_no_warn(&self) -> Option<&Box<Self>> {
        if let Self::NoWarn(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn try_into_no_warn(self) -> Result<Box<Self>, Self> {
        if let Self::NoWarn(v) = self {
            Ok(v)
        } else {
            Err(self)
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
    KeepForAll,
}

struct ReporterState<'patch, 'tx> {
    tx: &'tx mut mpsc::Sender<Msg>,
    missing: HashSet<&'patch Patch>,
    good: HashSet<&'patch Patch>,
    bad: HashSet<&'patch Patch>,
    pending: Vec<PendingAction<'patch>>,
    alloc: Allocator,
}

impl<'patch, 'tx> ReporterState<'patch, 'tx> {
    fn new(plugins: &'patch [Plugin], tx: &'tx mut mpsc::Sender<Msg>) -> Self {
        let missing: HashSet<&Patch> = plugins.iter().flat_map(|p| p.patches.iter()).collect();
        Self {
            tx,
            good: HashSet::with_capacity(missing.len()),
            bad: HashSet::new(),
            pending: Vec::new(),
            alloc: Allocator::new(),
            missing,
        }
    }

    fn process_module(&mut self, m_id: u32, m_txt: &str) {
        self.alloc.reset();
        let mut stats = None;
        let m_filename = format!("{m_id}.js");

        self.process_missing_patches(m_id, m_txt, &m_filename, &mut stats);
        self.process_good_patches(m_id, m_txt);
        self.process_bad_patches(m_id, m_txt);

        self.apply_pending_actions();
        self.pending.clear();
        self.send_progress();
    }

    fn process_missing_patches(
        &mut self,
        m_id: u32,
        m_txt: &str,
        m_filename: &str,
        stats: &mut Option<Stats>,
    ) {
        let patches: Vec<&Patch> = self.missing.iter().copied().collect();
        for patch in patches {
            if let Some(action) = self.evaluate_missing_patch(m_id, m_txt, m_filename, patch, stats)
            {
                self.pending.push(action);
            }
        }
    }

    fn evaluate_missing_patch(
        &mut self,
        m_id: u32,
        m_txt: &str,
        m_filename: &str,
        patch: &'patch Patch,
        stats: &mut Option<Stats>,
    ) -> Option<PendingAction<'patch>> {
        let plugin_id = patch.plugin_id();
        let matches = match matches_module(Some(self.tx), m_txt, patch, plugin_id) {
            Some(matches) => matches,
            None => return Some(PendingAction::MissingToBad(patch)),
        };

        if !matches {
            return None;
        }

        let mut last_src = format!("0,{m_txt}");
        let mut action = if patch.all {
            PendingAction::MissingToGood(patch)
        } else {
            PendingAction::KeepForAll
        };

        for r in &patch.replacement {
            let no_warn = patch.no_warn || r.no_warn;
            let pat = match &r.match_.v {
                Match::Str(_) => {
                    unreachable!()
                }
                Match::Regex(v) => match v.regex() {
                    Ok(r) => r,
                    Err(e) => {
                        self.tx
                            .blocking_send(
                            ReporterError::BadRegexSyntax {
                                plugin_id,
                                source: anyhow!("{e:?}"),
                                regex_span: r.match_.s.into(),
                                expanded: format!("/{}/{}", v.pattern, v.flags),
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
            if it.next().is_none() {
                let mut err = ReporterError::ReplaceMatchNotFound {
                    module_id: m_id,
                    plugin_id,
                    match_span: r.match_.s.into(),
                };
                if no_warn {
                    err = ReporterError::NoWarn(Box::new(err));
                }
                self.tx.blocking_send(err.into()).unwrap();
                if !no_warn {
                    action = PendingAction::MissingToBad(patch);
                }
                continue;
            }

            if it.next().is_some() {
                self.tx
                    .blocking_send(
                    ReporterError::ReplaceMatchAmbiguous {
                        module_id: m_id,
                        plugin_id,
                        match_span: r.match_.s.into(),
                    }
                    .into(),
                )
                    .unwrap();
            }

            let new_src = match &r.replace.v {
                Replacer::Str(s) => pat.replace(&last_src, s.as_str()),
            };

            let chk = {
                let chk = check_syntax_errors(&self.alloc, &new_src, *stats, Some(m_filename));
                self.alloc.reset();
                chk
            };
            match chk {
                Ok(s) => *stats = Some(s),
                Err(e) => {
                    self.tx
                        .blocking_send(
                        ReporterError::ReplaceSyntaxError {
                            module_id: m_id,
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

        Some(action)
    }

    fn process_good_patches(&mut self, m_id: u32, m_txt: &str) {
        let patches: Vec<&Patch> = self.good.iter().copied().collect();
        for patch in patches {
            if patch.all {
                continue;
            }

            if matches_module(None, m_txt, patch, patch.plugin_id()).unwrap() {
                self.tx
                    .blocking_send(
                    ReporterError::FindAmbiguous {
                        module_id: m_id,
                        plugin_id: patch.plugin_id(),
                        find_span: patch.find.s.into(),
                    }
                    .into(),
                )
                    .unwrap();
                self.pending.push(PendingAction::GoodToBad(patch));
            }
        }
    }

    fn process_bad_patches(&mut self, m_id: u32, m_txt: &str) {
        let patches: Vec<&Patch> = self.bad.iter().copied().collect();
        for patch in patches {
            if patch.all {
                continue;
            }

            if matches_module(None, m_txt, patch, patch.plugin_id()) == Some(true) {
                self.tx
                    .blocking_send(
                    ReporterError::FindAmbiguous {
                        module_id: m_id,
                        plugin_id: patch.plugin_id(),
                        find_span: patch.find.s.into(),
                    }
                    .into(),
                )
                    .unwrap();
            }
        }
    }

    fn apply_pending_actions(&mut self) {
        for action in &self.pending {
            match action {
                PendingAction::GoodToBad(p) => {
                    if !p.all {
                        debug_assert!(
                            self.good.contains(p),
                            "GoodToBad action on patch that is not in good set"
                        );
                        debug_assert!(
                            !self.bad.contains(p),
                            "GoodToBad action on patch that is already in bad set"
                        );
                    }
                    self.good.remove(p);
                    self.bad.insert(p);
                }
                PendingAction::MissingToGood(p) => {
                    if !p.all {
                        debug_assert!(
                            self.missing.contains(p),
                            "MissingToGood action on patch that is not in missing set"
                        );
                        debug_assert!(
                            !self.good.contains(p),
                            "MissingToGood action on patch that is already in good set"
                        );
                        self.missing.remove(p);
                    }
                    self.good.insert(p);
                }
                PendingAction::MissingToBad(p) => {
                    if !p.all {
                        debug_assert!(
                            self.missing.contains(p),
                            "MissingToBad action on patch that is not in missing set"
                        );
                        debug_assert!(
                            !self.bad.contains(p),
                            "MissingToBad action on patch that is already in bad set"
                        );
                        self.missing.remove(p);
                    }
                    self.bad.insert(p);
                }
                PendingAction::KeepForAll => {}
            }
        }
    }

    fn send_progress(&self) {
        match self.tx.try_send(Msg::Progress) {
            Ok(()) => {}
            Err(e) => {
                warn!("Failed to send progress update: {e}");
            }
        }
    }
}

fn run_reporter(build: &FullBundle, plugins: &[Plugin], tx: &mut mpsc::Sender<Msg>) {
    let mut state = ReporterState::new(plugins, tx);
    for (m_id, m_txt) in &build.modules {
        state.process_module(*m_id, m_txt);
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
                                expanded: format!("/{}/{}", s.pattern, s.flags),
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
