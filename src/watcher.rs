use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::scanner;

/// Spawns a background watcher on `~/.claude/sessions/` (non-recursive),
/// `~/.claude/projects/` (recursive), and `~/.codex/sessions/` (recursive) so
/// both Claude and Codex sessions update live. Sends a unit signal to `tx`
/// whenever any relevant event arrives. The receiver is expected to debounce.
///
/// Returns true if at least one watch was established. False means the caller
/// should fall back to a faster polling cadence.
pub fn spawn(tx: Sender<()>) -> bool {
    let claude = scanner::claude_dir();
    let sessions_dir: PathBuf = claude.join("sessions");
    let projects_dir: PathBuf = claude.join("projects");
    let codex_sessions: PathBuf = scanner::codex_dir().join("sessions");
    let _ = std::fs::create_dir_all(&sessions_dir);
    let _ = std::fs::create_dir_all(&projects_dir);

    let (raw_tx, raw_rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher: RecommendedWatcher = match Watcher::new(
        move |res| {
            let _ = raw_tx.send(res);
        },
        Config::default().with_poll_interval(Duration::from_secs(2)),
    ) {
        Ok(w) => w,
        Err(_) => return false,
    };

    let mut any_ok = false;
    if watcher
        .watch(&sessions_dir, RecursiveMode::NonRecursive)
        .is_ok()
    {
        any_ok = true;
    }
    if watcher
        .watch(&projects_dir, RecursiveMode::Recursive)
        .is_ok()
    {
        any_ok = true;
    }
    // Codex sessions live-update too. Only watch if the dir exists so we don't
    // create ~/.codex for users who don't use Codex.
    if codex_sessions.is_dir()
        && watcher
            .watch(&codex_sessions, RecursiveMode::Recursive)
            .is_ok()
    {
        any_ok = true;
    }
    if !any_ok {
        return false;
    }

    thread::spawn(move || {
        // Keep watcher alive for the lifetime of this thread.
        let _watcher = watcher;
        while let Ok(ev) = raw_rx.recv() {
            let Ok(ev) = ev else { continue };
            if matches!(
                ev.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                let _ = tx.send(());
            }
        }
    });

    true
}
