use chrono::Local;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub archived: Option<PathBuf>,
    pub moved: bool,
    pub note: Option<String>,
}

pub fn resolve_destination_path(
    watch_dir: &Path,
    destination: &str,
    allow_external: bool,
) -> Result<PathBuf, String> {
    let trimmed = destination.trim();
    if trimmed.is_empty() {
        return Err("destination is empty".to_string());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        if path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err("destination may not contain '..'".to_string());
        }
        let inside_watch = path.starts_with(watch_dir);
        if !allow_external && !inside_watch {
            return Err("external destinations are disabled".to_string());
        }
        return Ok(path.to_path_buf());
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("destination may not contain '..'".to_string());
    }
    Ok(watch_dir.join(path))
}

pub fn move_file(
    source: &Path,
    destination_dir: &Path,
    policy: &str,
    timestamp_prefix: bool,
) -> Result<MoveOutcome, String> {
    if !source.exists() {
        return Err(format!("source no longer exists: {}", source.display()));
    }
    fs::create_dir_all(destination_dir)
        .map_err(|e| format!("cannot create {}: {e}", destination_dir.display()))?;

    let file_name = source
        .file_name()
        .ok_or_else(|| "source has no file name".to_string())?
        .to_os_string();

    let mut destination = destination_dir.join(&file_name);
    let mut archived: Option<PathBuf> = None;

    match policy {
        "replace" => {
            if destination.exists() {
                fs::remove_file(&destination)
                    .map_err(|e| format!("cannot replace {}: {e}", destination.display()))?;
            }
        }
        "rename" => {
            if destination.exists() {
                destination = unique_path(destination_dir, &file_name);
            }
        }
        "skip" => {
            if destination.exists() {
                return Ok(MoveOutcome {
                    source: source.to_path_buf(),
                    destination,
                    archived: None,
                    moved: false,
                    note: Some("destination already exists; skipped".to_string()),
                });
            }
        }
        _ => {
            if destination.exists() {
                let previous_dir = destination_dir.join("Previous Versions");
                fs::create_dir_all(&previous_dir)
                    .map_err(|e| format!("cannot create {}: {e}", previous_dir.display()))?;
                let previous_name = if timestamp_prefix {
                    timestamped_name(&file_name)
                } else {
                    file_name.clone()
                };
                let previous_path = unique_path(&previous_dir, &previous_name);
                fs::rename(&destination, &previous_path)
                    .map_err(|e| format!("cannot archive previous version: {e}"))?;
                archived = Some(previous_path);
            }
        }
    }

    rename_or_copy(source, &destination)?;

    Ok(MoveOutcome {
        source: source.to_path_buf(),
        destination,
        archived,
        moved: true,
        note: None,
    })
}

fn rename_or_copy(source: &Path, destination: &Path) -> Result<(), String> {
    if fs::rename(source, destination).is_ok() {
        return Ok(());
    }
    fs::copy(source, destination)
        .map_err(|e| format!("cannot copy to {}: {e}", destination.display()))?;
    let source_len = fs::metadata(source).map(|m| m.len()).unwrap_or(0);
    let destination_len = fs::metadata(destination).map(|m| m.len()).unwrap_or(0);
    if source_len != destination_len {
        let _ = fs::remove_file(destination);
        return Err("copy verification failed".to_string());
    }
    fs::remove_file(source).map_err(|e| format!("cannot remove source: {e}"))?;
    Ok(())
}

fn unique_path(dir: &Path, file_name: &OsString) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let name = file_name.to_string_lossy().to_string();
    let (stem, extension) = match name.rfind('.') {
        Some(index) if index > 0 => (&name[..index], &name[index..]),
        _ => (name.as_str(), ""),
    };
    let mut counter = 1;
    loop {
        let candidate = dir.join(format!("{stem}_{counter}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

fn timestamped_name(file_name: &OsString) -> OsString {
    let stamp = Local::now().format("%Y-%m-%d").to_string();
    OsString::from(format!("{stamp}_{}", file_name.to_string_lossy()))
}
