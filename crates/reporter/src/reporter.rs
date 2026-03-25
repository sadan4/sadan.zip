use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{util::Stage, vc::{Match, Patch, Plugin, Replacer}};
use anyhow::{Result, anyhow};
use derive_more::{Deref, DerefMut, From, Into, IsVariant, TryUnwrap};
use explorer_types::FullBundle;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use memchr::memmem::find;
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
    ProgressBar(ProgressBar),
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
    patches: HashSet<&'a Patch>,
    find_map: HashMap<&'a Patch, Vec<u32>>,
    alloc: Allocator,
    build: &'a FullBundle,
    stats: HashMap<u32, Option<Stats>>,
    file_names: HashMap<u32, String>,
}

impl<'a> ReporterState<'a> {
    fn new(plugins: &'a [Plugin], build: &'a FullBundle, tx: &'a mut mpsc::Sender<Msg>) -> Self {
        let patches: HashSet<&Patch> = plugins.iter().flat_map(|p| p.patches.iter()).collect();
        let mut stats: HashMap<_, _> = build.modules.keys().map(|&m_id| (m_id, None)).collect();
        let mut file_names: HashMap<_, _> = build
            .modules
            .keys()
            .map(|&m_id| (m_id, format!("{m_id}.js")))
            .collect();
        let mut find_map: HashMap<_, _> = patches.iter().map(|&p| (p, Vec::new())).collect();
        stats.shrink_to_fit();
        file_names.shrink_to_fit();
        find_map.shrink_to_fit();
        Self {
            tx,
            build,
            patches,
            stats,
            file_names,
            find_map,
            alloc: Allocator::new(),
        }
    }
}

#[allow(clippy::multiple_inherent_impl)]
impl<'a> ReporterState<'a> {
    fn run(mut self) {
        self.prune_bad_finds();
        self.collect_finds();
    }
    #[must_use = "RAII guard"]
    fn stage(&mut self, msg: &'static str, n: Option<u32>) -> Stage {
        let stage = Stage::new(msg, n);
        self.tx
            .blocking_send(Msg::ProgressBar(stage.0.clone()))
            .unwrap();
        stage
    }
    fn prune_bad_finds(&mut self) {
        let _ = self.stage("Pruning bad finds", None);
        self.patches.retain(|p| {
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
        let progress = self.stage(
            "Collecting find matches",
            Some(self.build.modules.len() as _),
        );
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
}

// impl <'a> ReporterState<'a> {
//     fn process_module(&mut self, m_id: u32, m_txt: &str) {
//         self.alloc.reset();
//         let mut stats = None;
//         let m_filename = format!("{m_id}.js");

//         self.process_missing_patches(m_id, m_txt, &m_filename, &mut stats);
//         self.process_good_patches(m_id, m_txt);
//         self.process_bad_patches(m_id, m_txt);

//         self.apply_pending_actions();
//         self.pending.clear();
//         self.send_progress();
//     }

//     fn process_missing_patches(
//         &mut self,
//         m_id: u32,
//         m_txt: &str,
//         m_filename: &str,
//         stats: &mut Option<Stats>,
//     ) {
//         let patches: Vec<&Patch> = self.missing.iter().copied().collect();
//         for patch in patches {
//             if let Some(action) = self.evaluate_missing_patch(m_id, m_txt, m_filename, patch, stats)
//             {
//                 self.pending.push(action);
//             }
//         }
//     }

//     fn evaluate_missing_patch(
//         &mut self,
//         m_id: u32,
//         m_txt: &str,
//         m_filename: &str,
//         patch: &'a Patch,
//         stats: &mut Option<Stats>,
//     ) -> Option<PendingAction<'a>> {
//         let plugin_id = patch.plugin_id();
//         let Some(matches) = matches_module(Some(self.tx), m_txt, patch, plugin_id) else {
//             return Some(PendingAction::MissingToBad(patch));
//         };

//         if !matches {
//             return None;
//         }

//         let mut last_src = format!("0,{m_txt}");
//         let mut action = if patch.all {
//             PendingAction::MissingToGood(patch)
//         } else {
//             PendingAction::KeepForAll
//         };

//         for r in &patch.replacement {
//             let no_warn = patch.no_warn || r.no_warn;
//             let pat = match &r.match_.v {
//                 Match::Str(_) => {
//                     unreachable!()
//                 }
//                 Match::Regex(v) => match v.regex() {
//                     Ok(r) => r,
//                     Err(e) => {
//                         self.tx
//                             .blocking_send(
//                                 ReporterError::BadRegexSyntax {
//                                     plugin_id,
//                                     source: anyhow!("{e:?}"),
//                                     regex_span: r.match_.s.into(),
//                                     expanded: format!("/{}/{}", v.pattern, v.flags),
//                                 }
//                                 .into(),
//                             )
//                             .unwrap();
//                         action = PendingAction::MissingToBad(patch);
//                         continue;
//                     }
//                 },
//             };

//             let mut it = pat.find_iter(&last_src);
//             if it.next().is_none() {
//                 let mut err = ReporterError::ReplaceMatchNotFound {
//                     module_id: m_id,
//                     plugin_id,
//                     match_span: r.match_.s.into(),
//                 };
//                 if no_warn {
//                     err = ReporterError::NoWarn(Box::new(err));
//                 }
//                 self.tx.blocking_send(err.into()).unwrap();
//                 if !no_warn {
//                     action = PendingAction::MissingToBad(patch);
//                 }
//                 continue;
//             }

//             if it.next().is_some() {
//                 self.tx
//                     .blocking_send(
//                         ReporterError::ReplaceMatchAmbiguous {
//                             module_id: m_id,
//                             plugin_id,
//                             match_span: r.match_.s.into(),
//                         }
//                         .into(),
//                     )
//                     .unwrap();
//             }

//             let new_src = match &r.replace.v {
//                 Replacer::Str(s) => pat.replace(&last_src, s.as_str()),
//             };

//             let chk = {
//                 let chk = check_syntax_errors(&self.alloc, &new_src, *stats, Some(m_filename));
//                 self.alloc.reset();
//                 chk
//             };
//             match chk {
//                 Ok(s) => *stats = Some(s),
//                 Err(e) => {
//                     self.tx
//                         .blocking_send(
//                             ReporterError::ReplaceSyntaxError {
//                                 module_id: m_id,
//                                 plugin_id,
//                                 replace_span: r.replace.s.into(),
//                                 cause: e,
//                             }
//                             .into(),
//                         )
//                         .unwrap();
//                     action = PendingAction::MissingToBad(patch);
//                     continue;
//                 }
//             }

//             last_src = new_src;
//         }

//         Some(action)
//     }

//     fn process_good_patches(&mut self, m_id: u32, m_txt: &str) {
//         let patches: Vec<&Patch> = self.good.iter().copied().collect();
//         for patch in patches {
//             if patch.all {
//                 continue;
//             }

//             if matches_module(None, m_txt, patch, patch.plugin_id()).unwrap() {
//                 self.tx
//                     .blocking_send(
//                         ReporterError::FindAmbiguous {
//                             module_id: m_id,
//                             plugin_id: patch.plugin_id(),
//                             find_span: patch.find.s.into(),
//                         }
//                         .into(),
//                     )
//                     .unwrap();
//                 self.pending.push(PendingAction::GoodToBad(patch));
//             }
//         }
//     }

//     fn process_bad_patches(&self, m_id: u32, m_txt: &str) {
//         let patches: Vec<&Patch> = self.bad.iter().copied().collect();
//         for patch in patches {
//             if patch.all {
//                 continue;
//             }

//             if matches_module(None, m_txt, patch, patch.plugin_id()) == Some(true) {
//                 self.tx
//                     .blocking_send(
//                         ReporterError::FindAmbiguous {
//                             module_id: m_id,
//                             plugin_id: patch.plugin_id(),
//                             find_span: patch.find.s.into(),
//                         }
//                         .into(),
//                     )
//                     .unwrap();
//             }
//         }
//     }

//     fn apply_pending_actions(&mut self) {
//         for action in &self.pending {
//             match action {
//                 PendingAction::GoodToBad(p) => {
//                     if !p.all {
//                         debug_assert!(
//                             self.good.contains(p),
//                             "GoodToBad action on patch that is not in good set"
//                         );
//                         debug_assert!(
//                             !self.bad.contains(p),
//                             "GoodToBad action on patch that is already in bad set"
//                         );
//                     }
//                     self.good.remove(p);
//                     self.bad.insert(p);
//                 }
//                 PendingAction::MissingToGood(p) => {
//                     if !p.all {
//                         debug_assert!(
//                             self.missing.contains(p),
//                             "MissingToGood action on patch that is not in missing set"
//                         );
//                         debug_assert!(
//                             !self.good.contains(p),
//                             "MissingToGood action on patch that is already in good set"
//                         );
//                         self.missing.remove(p);
//                     }
//                     self.good.insert(p);
//                 }
//                 PendingAction::MissingToBad(p) => {
//                     if !p.all {
//                         debug_assert!(
//                             self.missing.contains(p),
//                             "MissingToBad action on patch that is not in missing set"
//                         );
//                         debug_assert!(
//                             !self.bad.contains(p),
//                             "MissingToBad action on patch that is already in bad set"
//                         );
//                         self.missing.remove(p);
//                     }
//                     self.bad.insert(p);
//                 }
//                 PendingAction::KeepForAll => {}
//             }
//         }
//     }

//     fn send_progress(&self) {
//         match self.tx.try_send(Msg::Progress) {
//             Ok(()) => {}
//             Err(e) => {
//                 warn!("Failed to send progress update: {e}");
//             }
//         }
//     }
// }

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
