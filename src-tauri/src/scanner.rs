use crate::classifier;
use crate::config::Config;
use crate::state::{AppState, QueueItem};
use chrono::Local;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
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

pub fn run(handle: AppHandle) {
    let mut tracked: HashMap<PathBuf, (u64, u32)> = HashMap::new();
    loop {
        let interval = {
            let state = handle.state::<AppState>();
            let config = state.config.lock().unwrap();
            config.scan_interval_ms.max(250)
        };
        std::thread::sleep(Duration::from_millis(interval));
        scan_once(&handle, &mut tracked);
    }
}

fn scan_once(handle: &AppHandle, tracked: &mut HashMap<PathBuf, (u64, u32)>) {
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

    let mut present: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

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
        let tracked_entry = tracked.entry(path.clone()).or_insert((size, 0));
        if tracked_entry.0 == size {
            tracked_entry.1 += 1;
        } else {
            tracked_entry.0 = size;
            tracked_entry.1 = 0;
        }
        if tracked_entry.1 >= 1 {
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
    let _ = handle.emit("file:detected", &item);
    crate::emit_queue(handle);
}

fn set_last_scan(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    *state.last_scan.lock().unwrap() = Some(Local::now().to_rfc3339());
}
