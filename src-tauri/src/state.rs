use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub path: PathBuf,
    pub file_name: String,
    pub ext: String,
    pub size: u64,
    pub suggested: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub archived: Option<PathBuf>,
    pub rule: Option<String>,
    pub timestamp: String,
    pub undone: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub watch_dir: String,
    pub pending: usize,
    pub paused: bool,
    pub scanning: bool,
    pub last_scan: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizerItem {
    pub path: PathBuf,
    pub file_name: String,
    pub ext: String,
    pub size: u64,
    pub suggested: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeRequest {
    pub path: String,
    pub destination: String,
    #[serde(default)]
    pub remember: bool,
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub queue: Mutex<Vec<QueueItem>>,
    pub seen: Mutex<HashSet<PathBuf>>,
    pub history: Mutex<Vec<HistoryEntry>>,
    pub app_start: SystemTime,
    pub last_scan: Mutex<Option<String>>,
}
