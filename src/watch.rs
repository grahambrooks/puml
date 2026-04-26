//! Filesystem-driven re-render loop for `--watch`.
//!
//! Strategy: render once up front, then ask `notify-debouncer-mini` to
//! coalesce filesystem events into batched ticks. On each tick we re-run the
//! whole preprocess→render pipeline. Because preprocess returns the set of
//! `!include`'d files, we re-subscribe to that set after every render — so a
//! newly-added include starts being watched the next time the parent file
//! references it.
//!
//! Errors during re-render are printed and the loop continues; the user
//! probably wants to fix them in the next save rather than have the watcher
//! die on the first typo.

use anyhow::{anyhow, Context, Result};
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

const DEBOUNCE: Duration = Duration::from_millis(100);

pub fn run(input: &Path, output: &Path, args: &crate::cli::Args) -> Result<()> {
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();
    let mut debouncer: Debouncer<RecommendedWatcher> = new_debouncer(DEBOUNCE, move |res| {
        // Best-effort send: if the receiver is gone we're shutting down
        // and dropping the event is the right call.
        let _ = tx.send(res);
    })
    .context("creating filesystem debouncer")?;

    let mut watched: HashSet<PathBuf> = HashSet::new();
    let initial = render_and_report(input, output, args);
    update_watch_set(&mut debouncer, &mut watched, input, &initial);

    eprintln!("puml: watching {:?} → {:?}", input, output);

    loop {
        match rx.recv() {
            Ok(Ok(_events)) => {
                let includes = render_and_report(input, output, args);
                update_watch_set(&mut debouncer, &mut watched, input, &includes);
            }
            Ok(Err(e)) => {
                eprintln!("puml: watcher error: {e}");
            }
            Err(_) => return Err(anyhow!("watcher channel closed")),
        }
    }
}

/// Run one render cycle, printing the outcome to stderr. Returns the include
/// set so the watcher can refresh subscriptions; on error returns an empty
/// vec — we'll still re-watch the input itself.
fn render_and_report(input: &Path, output: &Path, args: &crate::cli::Args) -> Vec<PathBuf> {
    let source = match std::fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("puml: read {:?}: {e}", input);
            return Vec::new();
        }
    };
    let base_dir = input.parent().map(|p| p.to_path_buf());
    match crate::render_once(&source, base_dir.as_deref(), Some(output), args) {
        Ok(includes) => {
            eprintln!("puml: wrote {:?}", output);
            includes
        }
        Err(e) => {
            eprintln!("puml: error: {e:#}");
            Vec::new()
        }
    }
}

/// Sync the watcher's subscription set with the current `{input} ∪ includes`.
/// Adds new files, removes ones that have dropped out of the include graph.
/// `notify-debouncer-mini` is forgiving about both operations failing on a
/// missing file, so we just log and move on.
fn update_watch_set(
    debouncer: &mut Debouncer<RecommendedWatcher>,
    watched: &mut HashSet<PathBuf>,
    input: &Path,
    includes: &[PathBuf],
) {
    let mut desired: HashSet<PathBuf> = includes.iter().cloned().collect();
    desired.insert(input.to_path_buf());

    for added in desired.difference(watched) {
        if let Err(e) = debouncer
            .watcher()
            .watch(added, RecursiveMode::NonRecursive)
        {
            eprintln!("puml: watch {:?}: {e}", added);
        }
    }
    for removed in watched.difference(&desired) {
        let _ = debouncer.watcher().unwatch(removed);
    }
    *watched = desired;
}
