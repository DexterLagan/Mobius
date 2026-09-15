use crate::classifier;
use crate::config::Config;
use crate::state::{AppState, QueueItem};
use chrono::Local;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

const INCOMPLETE_EXTENSIONS: [&str; 8] = [
    "crdownload",
    "part",
    "partial",
    "download",
    "tmp",
    "opdownload",
    "fdmdownload",
    "aria2",
];

/// A file must keep the same size for this long before it is considered finished.
const STABLE_MS: u64 = 1500;

struct Tracked {
    size: u64,
    changed_at: Instant,
}

pub fn run(handle: AppHandle) {
    let mut tracked: HashMap<PathBuf, Tracked> = HashMap::new();
    let mut _watcher: Option<RecommendedWatcher> = None;
    let mut receiver: Option<Receiver<()>> = None;
    let mut watched_dir: Option<PathBuf> = None;

    loop {
        let (interval, debounce, watch_dir, paused) = {
            let state = handle.state::<AppState>();
            let config = state.config.lock().unwrap();
            let paused = config
                .paused_until
                .map(|until| until > Local::now().timestamp())
                .unwrap_or(false);
            (
                config.scan_interval_ms.max(250),
                config.debounce_ms.max(50),
                config.watch_dir.clone(),
                paused,
            )
        };

        if watched_dir.as_deref() != Some(watch_dir.as_path()) {
            match create_watcher(&watch_dir) {
                Ok((new_watcher, new_receiver)) => {
                    log::info!("watching {} for filesystem events", watch_dir.display());
                    _watcher = Some(new_watcher);
                    receiver = Some(new_receiver);
                }
                Err(err) => {
                    log::warn!(
                        "filesystem events unavailable for {}: {err}; using timed scan only",
                        watch_dir.display()
                    );
                    _watcher = None;
                    receiver = None;
                }
            }
            watched_dir = Some(watch_dir);
        }

        if paused {
            std::thread::sleep(Duration::from_millis(interval));
            continue;
        }

        let should_scan = match receiver.as_ref() {
            Some(rx) => match rx.recv_timeout(Duration::from_millis(interval)) {
                Ok(()) => {
                    drain_burst(rx, debounce);
                    true
                }
                Err(RecvTimeoutError::Timeout) => true,
                Err(RecvTimeoutError::Disconnected) => {
                    log::warn!("filesystem watcher stopped; using timed scan only");
                    _watcher = None;
                    receiver = None;
                    std::thread::sleep(Duration::from_millis(interval));
                    true
                }
            },
            None => {
                std::thread::sleep(Duration::from_millis(interval));
                true
            }
        };

        if should_scan {
            scan_once(&handle, &mut tracked);
        }
    }
}

fn create_watcher(dir: &Path) -> notify::Result<(RecommendedWatcher, Receiver<()>)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if result.is_ok() {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok((watcher, rx))
}

/// Collapse a burst of filesystem events into a single scan. Waits until the
/// events stop arriving or `debounce_ms` elapses, whichever comes first.
fn drain_burst(rx: &Receiver<()>, debounce_ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(debounce_ms);
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if rx.recv_timeout(remaining).is_err() {
            break;
        }
    }
}

fn scan_once(handle: &AppHandle, tracked: &mut HashMap<PathBuf, Tracked>) {
    let (config, watch_dir, app_start, seen) = {
        let state = handle.state::<AppState>();
        let config = state.config.lock().unwrap().clone();
        let seen = state.seen.lock().unwrap().clone();
        let watch_dir = config.watch_dir.clone();
        (config, watch_dir, state.app_start, seen)
    };

    if let Some(until) = config.paused_until {
        if until > Local::now().timestamp() {
            set_last_scan(handle);
            return;
        }
    }

    let entries = match std::fs::read_dir(&watch_dir) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!("cannot read {}: {err}", watch_dir.display());
            set_last_scan(handle);
            return;
        }
    };

    let now = Instant::now();
    let mut present: HashSet<PathBuf> = HashSet::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if !metadata.is_file() {
            continue;
        }
        if config.ignored_names.iter().any(|ignored| ignored == &name) {
            continue;
        }
        let ext = classifier::extension_of(&name);
        if !ext.is_empty()
            && config
                .ignored_extensions
                .iter()
                .any(|ignored| ignored.eq_ignore_ascii_case(&ext))
        {
            continue;
        }
        if !ext.is_empty() && INCOMPLETE_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        if seen.contains(&path) {
            continue;
        }
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if modified < app_start {
            continue;
        }

        present.insert(path.clone());
        let size = metadata.len();
        let ready = match tracked.get_mut(&path) {
            Some(entry) => {
                if entry.size != size {
                    entry.size = size;
                    entry.changed_at = now;
                    false
                } else {
                    now.duration_since(entry.changed_at) >= Duration::from_millis(STABLE_MS)
                }
            }
            None => {
                tracked.insert(
                    path.clone(),
                    Tracked {
                        size,
                        changed_at: now,
                    },
                );
                false
            }
        };

        if ready {
            tracked.remove(&path);
            promote(handle, &config, &path, &name, &ext, size);
        }
    }

    tracked.retain(|path, _| present.contains(path));
    set_last_scan(handle);
}

fn promote(handle: &AppHandle, config: &Config, path: &Path, name: &str, ext: &str, size: u64) {
    let suggested = classifier::resolve_destination(config, ext);
    let item = QueueItem {
        id: Uuid::new_v4().to_string(),
        path: path.to_path_buf(),
        file_name: name.to_string(),
        ext: ext.to_string(),
        size,
        suggested,
        detected_at: Local::now().to_rfc3339(),
    };
    {
        let state = handle.state::<AppState>();
        state.seen.lock().unwrap().insert(path.to_path_buf());
        state.queue.lock().unwrap().push(item.clone());
    }
    log::info!("detected {} → suggested {}", name, item.suggested);
    let _ = handle.emit("file:detected", &item);
    crate::emit_queue(handle);
}

fn set_last_scan(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    *state.last_scan.lock().unwrap() = Some(Local::now().to_rfc3339());
}
