use std::{
    collections::HashSet, sync::Arc,
};

use crate::vc::{Match, Patch, Plugin};
use anyhow::{Result, anyhow};
use explorer_types::FullBundle;
use oxc::{
    allocator::Allocator,
    parser::Parser,
    semantic::{SemanticBuilder, Stats},
    span::{SourceType, Span},
};
use regress::{Regex, escape};
use tokio::{sync::mpsc, task};
use tracing::{debug, warn};

#[derive(Debug)]
pub enum Msg {
    Progress,
    BadRegexSyntax {
        plugin_id: u16,
        error: anyhow::Error,
    },
    ReplaceMatchNotFound {
        module_id: u32,
        plugin_id: u16,
        span: Span,
    },
    ReplaceMatchAmbiguous {
        module_id: u32,
        plugin_id: u16,
        span: Span,
    },
    ReplaceSyntaxError {
        module_id: u32,
        plugin_id: u16,
        span: Span,
        error: anyhow::Error,
    },
    FindAmbiguous {
        module_id: u32,
        plugin_id: u16,
        span: Span,
    },
    Done(Result<()>),
}

#[track_caller]
pub fn report_broken_patches(
    target_build: Arc<FullBundle>,
    plugins: Vec<Plugin>,
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
                    Match::Regex(r) => match r.regex() {
                        Ok(r) => r,
                        Err(e) => {
                            tx.blocking_send(Msg::BadRegexSyntax {
                                plugin_id,
                                error: anyhow!("{e:?}"),
                            })
                            .unwrap();
                            action = PendingAction::MissingToBad(patch);
                            continue;
                        }
                    },
                };
                let mut it = pat.find_iter(&last_src);
                // TODO: handle group patches
                if it.next().is_none() {
                    tx.blocking_send(Msg::ReplaceMatchNotFound {
                        module_id: *m_id,
                        plugin_id,
                        span: r.match_.s,
                    })
                    .unwrap();
                    action = PendingAction::MissingToBad(patch);
                    continue;
                }

                if it.next().is_some() {
                    tx.blocking_send(Msg::ReplaceMatchAmbiguous {
                        module_id: *m_id,
                        plugin_id,
                        span: r.match_.s,
                    })
                    .unwrap();
                }

                let new_src = match &r.replace.v {
                    crate::vc::Replacer::Str(s) => pat.replace(&last_src, s.as_str()),
                };

                let chk = {
                    let chk = check_syntax_errors(&alloc, &new_src, stats);
                    alloc.reset();
                    chk
                };
                match chk {
                    Ok(s) => stats = Some(s),
                    Err(e) => {
                        tx.blocking_send(Msg::ReplaceSyntaxError {
                            module_id: *m_id,
                            plugin_id,
                            span: r.replace.s,
                            error: e,
                        })
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
                tx.blocking_send(Msg::FindAmbiguous {
                    module_id: *m_id,
                    plugin_id: patch.plugin_id(),
                    span: patch.find.s,
                })
                .unwrap();
                pending.push(PendingAction::GoodToBad(patch));
            }
        }
        for &patch in &bad {
            if patch.all {
                continue;
            }

            if matches_module(None, m_txt, patch, patch.plugin_id()) == Some(true) {
                tx.blocking_send(Msg::FindAmbiguous {
                    module_id: *m_id,
                    plugin_id: patch.plugin_id(),
                    span: patch.find.s,
                })
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
                        tx.blocking_send(Msg::BadRegexSyntax {
                            error: anyhow!("{e:?}"),
                            plugin_id,
                        })
                        .unwrap();
                    }
                    return None;
                }
            };
            s.find(m_txt).is_some()
        }
    })
}

fn check_syntax_errors(alloc: &Allocator, src: &str, stats: Option<Stats>) -> Result<Stats> {
    let mut p_ret = Parser::new(alloc, src, SourceType::unambiguous()).parse();
    if !p_ret.errors.is_empty() {
        return Err(p_ret.errors.swap_remove(0).into());
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
        Err(sema.errors.swap_remove(0).into())
    }
}
