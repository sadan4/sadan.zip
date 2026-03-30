use std::{
    borrow::Borrow as _,
    collections::{HashMap, HashSet},
    fmt::Display,
    mem,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    util::Stage,
    vc::{Match, Patch, Plugin, Replacer},
};
use anyhow::{Result, anyhow};
use derive_more::{Deref, DerefMut, From, Into, IsVariant, TryUnwrap};
use explorer_types::FullBundle;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use itertools::Itertools as _;
use memchr::memmem::find;
use miette::{Diagnostic, NamedSource, Report, Severity, SourceSpan};
use oxc::{
    allocator::Allocator,
    ast::ast::{Program, RegExpFlags},
    codegen::{Codegen, CodegenOptions, CommentOptions, IndentChar, LegalComment},
    diagnostics::OxcDiagnostic,
    parser::Parser,
    semantic::{SemanticBuilder, Stats},
    span::{SourceType, Span},
};
use regress::{Regex, escape};
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot},
    task,
};
use tracing::{debug, warn};

#[derive(Debug)]
pub enum Msg {
    RequestProgressBar(oneshot::Sender<MultiProgress>),
    Error(ReporterError),
    Done(Result<Duration>),
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
        severity(Error),
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
        severity(Error),
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
        severity(Error),
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
        code(reporter::find::ambiguous),
        severity(Error),
        help("Modules {ok_ids:?} matched and applied without issue.\nModules {err_ids:?} matches, but errored while applying"),
    ]]
    // TODO: Add related failures here something like Option<Vec<ReporterError>>
    FindAmbiguous {
        #[label("This find matches more than one module. Make it more specific!")]
        find_span: SourceSpan,
        plugin_id: u16,
        ok_ids: Vec<u32>,
        err_ids: Vec<u32>,
    },
    #[error("Find Too Broad")]
    #[diagnostic[
        code(reporter::find::broad),
        severity(Warning),
        help("This patch executed without issue on module {ok_id}; however, it matched and failed to execute on modules {err_ids:?}.{extra_help}"),
    ]]
    FindAmbiguousRecoverable {
        #[label("This find matches more than one module. Make it more specific!")]
        find_span: SourceSpan,
        plugin_id: u16,
        ok_id: u32,
        err_ids: Vec<u32>,
        extra_help: &'static str,
    },
    #[error("No matches found")]
    #[diagnostic[
        code(reporter::find::not_found),
        severity(Error),
    ]]
    FindNotFound {
        #[label("This find failed to match anything")]
        find_span: SourceSpan,
        plugin_id: u16,
    },
    #[error(transparent)]
    NoWarn(Box<Self>),
}

impl ReporterError {
    pub const fn plugin_id(&self) -> u16 {
        match self {
            Self::BadRegexSyntax { plugin_id, .. }
            | Self::ReplaceMatchNotFound { plugin_id, .. }
            | Self::ReplaceMatchAmbiguous { plugin_id, .. }
            | Self::ReplaceSyntaxError { plugin_id, .. }
            | Self::FindNotFound { plugin_id, .. }
            | Self::FindAmbiguous { plugin_id, .. }
            | Self::FindAmbiguousRecoverable { plugin_id, .. } => *plugin_id,
            Self::NoWarn(e) => e.plugin_id(),
        }
    }

    pub const fn module_id(&self) -> Option<u32> {
        match self {
            Self::FindNotFound { .. }
            | Self::BadRegexSyntax { .. }
            | Self::FindAmbiguous { .. } => None,
            Self::ReplaceSyntaxError { module_id, .. }
            | Self::ReplaceMatchAmbiguous { module_id, .. }
            | Self::FindAmbiguousRecoverable {
                ok_id: module_id, ..
            }
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
    const BUFFER_SIZE: usize = 0x4000;
    let (mut tx, rx) = mpsc::channel(BUFFER_SIZE);
    task::spawn_blocking(move || {
        let start = Instant::now();
        run_reporter(&target_build, &plugins, &mut tx);
        let duration = start.elapsed();
        tx.blocking_send(Msg::Done(Ok(duration))).unwrap();
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

struct ReporterState<'a> {
    tx: &'a mut mpsc::Sender<Msg>,
    m_bar: MultiProgress,
    patches: HashSet<&'a Patch>,
    find_map: HashMap<&'a Patch, Vec<u32>>,
    alloc: Allocator,
    build: &'a FullBundle,
    stats: HashMap<u32, Stats>,
}

impl<'a> ReporterState<'a> {
    fn new(plugins: &'a [Plugin], build: &'a FullBundle, tx: &'a mut mpsc::Sender<Msg>) -> Self {
        let (pb_tx, rx) = oneshot::channel();
        tx.blocking_send(Msg::RequestProgressBar(pb_tx)).unwrap();
        let patches: HashSet<&Patch> = plugins.iter().flat_map(|p| p.patches.iter()).collect();
        let stats = HashMap::with_capacity(build.modules.len());
        let mut find_map: HashMap<_, _> = patches.iter().map(|&p| (p, Vec::new())).collect();
        find_map.shrink_to_fit();
        let m_bar = rx.blocking_recv().unwrap();
        Self {
            tx,
            m_bar,
            build,
            patches,
            stats,
            find_map,
            alloc: Allocator::new(),
        }
    }
}

#[derive(Copy, Clone, IsVariant)]
enum PatchStatus {
    Ok,
    Error,
}

#[allow(clippy::multiple_inherent_impl)]
impl<'a> ReporterState<'a> {
    fn run(mut self) {
        self.prune_bad_finds();
        self.collect_finds();
        self.report_empty_finds();
        self.resolve_ambiguous_finds();
        self.test_patches();
    }
    #[must_use = "RAII guard"]
    fn stage(&self, msg: &'static str, n: Option<usize>) -> Stage {
        Stage::new(msg, n).and_attach(&self.m_bar)
    }
    fn prune_bad_finds(&mut self) {
        let bar = self.stage("Pruning bad finds", Some(self.patches.len()));
        self.patches.retain(|p| {
            bar.step();
            if let Match::Regex(r) = &p.find.v
                && let Err(e) = r.regex()
            {
                self.tx
                    .blocking_send(
                        ReporterError::BadRegexSyntax {
                            plugin_id: p.plugin_id(),
                            source: anyhow!("{e:?}"),
                            regex_span: p.find.s.into(),
                            expanded: format!("/{}/{}", r.pattern, r.flags),
                        }
                        .into(),
                    )
                    .unwrap();
                false
            } else {
                true
            }
        });
    }
    fn collect_finds(&mut self) {
        let progress = self.stage("Collecting find matches", Some(self.build.modules.len()));
        for (&m_id, m_txt) in &self.build.modules {
            for patch in &self.patches {
                if matches_module(m_txt, patch) {
                    // this should never error because we pre-fill all the keys with empty vectors in the ctor
                    self.find_map.get_mut(patch).unwrap().push(m_id);
                }
            }
            progress.step();
        }
    }
    fn report_empty_finds(&mut self) {
        _ = self.stage("Reporting empty finds", None);
        for (patch, _) in self.find_map.extract_if(|_, patch| patch.is_empty()) {
            let mut err = ReporterError::FindNotFound {
                find_span: patch.find.s.into(),
                plugin_id: patch.plugin_id(),
            };
            if patch.no_warn {
                err = ReporterError::NoWarn(err.into());
            }
            self.tx.blocking_send(err.into()).unwrap();
        }
    }
    fn resolve_ambiguous_finds(&mut self) {
        let it = self
            .find_map
            .extract_if(|p, m| !p.all && m.len() > 1)
            .collect_vec();
        let bar = self.stage("Resolving ambiguous finds", Some(it.len()));
        for (patch, matches) in it {
            let mut failed = Vec::new();
            let mut good = Vec::new();
            for m_id in matches.iter().copied() {
                match self.test_patch_against_module(patch, m_id, None) {
                    PatchStatus::Ok => good.push(m_id),
                    PatchStatus::Error => failed.push(m_id),
                }
            }
            // TODO: suppress if patch is no_warn??
            let err = if good.len() == 1 {
                ReporterError::FindAmbiguousRecoverable {
                    find_span: patch.find.s.into(),
                    plugin_id: patch.plugin_id(),
                    ok_id: good[0],
                    extra_help: if failed.is_empty() {
                        "\nIf you intended for this patch to apply to all of the above modules, add the `all` property to the patch."
                    } else {
                        Default::default()
                    },
                    err_ids: failed,
                }
            } else {
                ReporterError::FindAmbiguous {
                    find_span: patch.find.s.into(),
                    plugin_id: patch.plugin_id(),
                    ok_ids: good,
                    err_ids: failed,
                }
            };
            self.tx.blocking_send(err.into()).unwrap();
            bar.step();
        }
    }
    fn test_patches(&mut self) {
        // temporarily take the find_map so we don't have to deal with 2x &mut self
        let found_patches = mem::take(&mut self.find_map);
        let bar = self.stage("Testing patches", Some(found_patches.len()));
        let mut errs = Vec::new();
        for (patch, ids) in &found_patches {
            for &m_id in ids {
                self.test_patch_against_module(patch, m_id, Some(&mut errs));
            }
            for err in errs.drain(..) {
                self.tx.blocking_send(err.into()).unwrap();
            }
            bar.step();
        }
        self.find_map = found_patches;
    }
    fn test_patch_against_module(
        &mut self,
        patch: &'a Patch,
        m_id: u32,
        mut errs: Option<&mut Vec<ReporterError>>,
    ) -> PatchStatus {
        let mut status = PatchStatus::Ok;
        let m_txt = self.build.modules.get(&m_id).expect("invalid module id");
        let mut last_src = format!("0,{m_txt}");
        let plugin_id = patch.plugin_id();
        let mut report = |e: ReporterError| {
            if !e.is_no_warn() && e.severity().is_none_or(|s| s == Severity::Error) {
                status = PatchStatus::Error;
            }
            if let Some(errs) = &mut errs {
                errs.push(e);
            }
        };

        for r in &patch.replacement {
            let no_warn = patch.no_warn || r.no_warn;
            let is_global = r
                .match_
                .v
                .as_regex()
                .is_some_and(|r| r.flags.contains(RegExpFlags::G));
            let pat = match &r.match_.v {
                Match::Str(_) => {
                    unreachable!()
                }
                Match::Regex(v) => match v.regex() {
                    Ok(r) => r,
                    Err(e) => {
                        report(ReporterError::BadRegexSyntax {
                            plugin_id,
                            source: anyhow!("{e:?}"),
                            regex_span: r.match_.s.into(),
                            expanded: format!("/{}/{}", v.pattern, v.flags),
                        });
                        continue;
                    }
                },
            };

            let mut it = pat.find_iter(last_src.as_str());

            if it.next().is_none() {
                let mut err = ReporterError::ReplaceMatchNotFound {
                    match_span: r.match_.s.into(),
                    module_id: m_id,
                    plugin_id,
                };
                if no_warn {
                    err = ReporterError::NoWarn(Box::new(err));
                }
                report(err);
                continue;
            }
            if !is_global && it.next().is_some() {
                report(ReporterError::ReplaceMatchAmbiguous {
                    match_span: r.match_.s.into(),
                    plugin_id,
                    module_id: m_id,
                });
            }

            let new_src = match &r.replace.v {
                Replacer::Str(s) => {
                    if is_global {
                        pat.replace_all(&last_src, s.as_str())
                    } else {
                        pat.replace(&last_src, s.as_str())
                    }
                }
            };

            let chk = {
                let chk = check_syntax_errors(
                    &self.alloc,
                    &new_src,
                    self.stats.get(&m_id).copied(),
                );
                self.alloc.reset();
                chk
            };

            match chk {
                Ok(s) => {
                    self.stats.entry(m_id).or_insert(s);
                }
                Err(e) => report(ReporterError::ReplaceSyntaxError {
                    replace_span: r.replace.s.into(),
                    cause: e,
                    module_id: m_id,
                    plugin_id,
                }),
            }

            last_src = new_src;
        }
        status
    }
}

fn run_reporter(build: &FullBundle, plugins: &[Plugin], tx: &mut mpsc::Sender<Msg>) {
    ReporterState::new(plugins, build, tx).run();
}

fn matches_module(m_txt: &str, patch: &Patch) -> bool {
    match &patch.find.v {
        Match::Str(s) => s.find(m_txt.as_bytes()).is_some(),
        Match::Regex(s) => {
            // we should never have a patch with bad regex
            // it should have been filtered out
            s.regex().as_ref().unwrap().find(m_txt).is_some()
        }
    }
}

fn check_syntax_errors(
    alloc: &Allocator,
    src: &str,
    stats: Option<Stats>,
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
