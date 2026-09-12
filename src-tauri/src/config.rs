use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

fn default_scan_interval() -> u64 {
    3000
}
fn default_batch_window() -> u64 {
    2000
}
fn default_debounce() -> u64 {
    750
}
fn default_duplicate_policy() -> String {
    "version".to_string()
}
fn default_undo_toast() -> u64 {
    10000
}
fn default_start_minimized() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub watch_dir: PathBuf,
    #[serde(default = "default_scan_interval")]
    pub scan_interval_ms: u64,
    #[serde(default = "default_batch_window")]
    pub batch_window_ms: u64,
    #[serde(default = "default_debounce")]
    pub debounce_ms: u64,
    #[serde(default = "default_duplicate_policy")]
    pub duplicate_policy: String,
    #[serde(default)]
    pub version_timestamp_prefix: bool,
    #[serde(default)]
    pub destination_map: HashMap<String, String>,
    #[serde(default)]
    pub category_destinations: HashMap<String, String>,
    #[serde(default)]
    pub flatten_to_categories: bool,
    #[serde(default)]
    pub allow_external_destinations: bool,
    #[serde(default)]
    pub ignored_extensions: Vec<String>,
    #[serde(default)]
    pub ignored_names: Vec<String>,
    #[serde(default)]
    pub rules: HashMap<String, String>,
    #[serde(default = "default_undo_toast")]
    pub undo_toast_ms: u64,
    #[serde(default)]
    pub auto_dismiss_timeout_ms: u64,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default = "default_start_minimized")]
    pub start_minimized_to_tray: bool,
    #[serde(default)]
    pub paused_until: Option<i64>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

impl Config {
    pub fn default_with_watch_dir(watch_dir: PathBuf) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            watch_dir,
            scan_interval_ms: default_scan_interval(),
            batch_window_ms: default_batch_window(),
            debounce_ms: default_debounce(),
            duplicate_policy: default_duplicate_policy(),
            version_timestamp_prefix: false,
            destination_map: HashMap::new(),
            category_destinations: HashMap::new(),
            flatten_to_categories: false,
            allow_external_destinations: false,
            ignored_extensions: Vec::new(),
            ignored_names: Vec::new(),
            rules: HashMap::new(),
            undo_toast_ms: default_undo_toast(),
            auto_dismiss_timeout_ms: 0,
            launch_at_login: false,
            start_minimized_to_tray: default_start_minimized(),
            paused_until: None,
        }
    }

    pub fn load(path: &Path) -> Self {
        let watch_dir = default_watch_dir();
        match fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<Config>(&raw) {
                Ok(config) => config,
                Err(err) => {
                    log::warn!("failed to parse config, using defaults: {err}");
                    let backup = path.with_extension("json.bak");
                    let _ = fs::rename(path, backup);
                    Config::default_with_watch_dir(watch_dir)
                }
            },
            Err(_) => Config::default_with_watch_dir(watch_dir),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("cannot create config dir: {e}"))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|e| format!("cannot serialize config: {e}"))?;
        fs::write(path, raw).map_err(|e| format!("cannot write config: {e}"))
    }
}

pub fn default_watch_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
}
