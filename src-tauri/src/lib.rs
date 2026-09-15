mod classifier;
mod config;
mod mover;
mod scanner;
mod state;

use classifier::resolve_destination;
use config::Config;
use mover::MoveOutcome;
use state::{AppState, HistoryEntry, OrganizeRequest, OrganizerItem, QueueItem, Status};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use uuid::Uuid;

fn now_ts() -> i64 {
    chrono::Local::now().timestamp()
}

pub fn emit_queue(app: &AppHandle) {
    let queue = app.state::<AppState>().queue.lock().unwrap().clone();
    let _ = app.emit("queue:updated", queue);
}

pub fn emit_history(app: &AppHandle) {
    let history = app.state::<AppState>().history.lock().unwrap().clone();
    let _ = app.emit("history:updated", history);
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_config(state: State<AppState>, config: Config) -> Result<(), String> {
    config.save(&state.config_path)?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
fn list_queue(state: State<AppState>) -> Vec<QueueItem> {
    state.queue.lock().unwrap().clone()
}

#[tauri::command]
fn list_history(state: State<AppState>) -> Vec<HistoryEntry> {
    state.history.lock().unwrap().clone()
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Status {
    let config = state.config.lock().unwrap().clone();
    let pending = state.queue.lock().unwrap().len();
    let last_scan = state.last_scan.lock().unwrap().clone();
    let paused = config
        .paused_until
        .map(|until| until > now_ts())
        .unwrap_or(false);
    Status {
        watch_dir: config.watch_dir.display().to_string(),
        pending,
        paused,
        scanning: true,
        last_scan,
    }
}

#[tauri::command]
fn dismiss(app: AppHandle, state: State<AppState>, path: String) {
    let target = PathBuf::from(&path);
    state
        .queue
        .lock()
        .unwrap()
        .retain(|item| item.path != target);
    emit_queue(&app);
}

#[tauri::command]
fn confirm_move(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    custom_destination: Option<String>,
    remember: bool,
    apply_all: bool,
) -> Result<Vec<MoveOutcome>, String> {
    let config = state.config.lock().unwrap().clone();
    let watch_dir = config.watch_dir.clone();

    let mut targets: Vec<(PathBuf, String, String)> = Vec::new();
    let mut learned_rule: Option<(String, String)> = None;

    if apply_all {
        let queue = state.queue.lock().unwrap().clone();
        for item in queue {
            let destination = custom_destination
                .clone()
                .unwrap_or_else(|| item.suggested.clone());
            targets.push((item.path, item.ext, destination));
        }
    } else {
        let target = PathBuf::from(&path);
        let existing = state
            .queue
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.path == target)
            .cloned();
        let (ext, suggested) = match existing {
            Some(item) => (item.ext, item.suggested),
            None => {
                let name = target
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let ext = classifier::extension_of(&name);
                let suggested = resolve_destination(&config, &ext);
                (ext, suggested)
            }
        };
        let destination = custom_destination
            .clone()
            .unwrap_or_else(|| suggested.clone());
        if remember {
            learned_rule = Some((ext.clone(), destination.clone()));
        }
        targets.push((target, ext, destination));
    }

    let mut outcomes = Vec::new();
    let mut moved_paths: HashSet<PathBuf> = HashSet::new();

    for (source, ext, destination) in targets {
        let destination_dir = match mover::resolve_destination_path(
            &watch_dir,
            &destination,
            config.allow_external_destinations,
        ) {
            Ok(dir) => dir,
            Err(err) => {
                let _ = app.emit("move:failed", format!("{err}: {}", source.display()));
                continue;
            }
        };

        match mover::move_file(
            &source,
            &destination_dir,
            &config.duplicate_policy,
            config.version_timestamp_prefix,
        ) {
            Ok(outcome) => {
                if outcome.moved {
                    moved_paths.insert(source.clone());
                }
                if let Some(rule) = &learned_rule {
                    if ext == rule.0 {
                        record_history(&app, &outcome, Some(rule.1.clone()));
                    } else {
                        record_history(&app, &outcome, None);
                    }
                } else {
                    record_history(&app, &outcome, None);
                }
                outcomes.push(outcome);
            }
            Err(err) => {
                let _ = app.emit("move:failed", format!("{err}: {}", source.display()));
            }
        }
    }

    if let Some((ext, destination)) = learned_rule {
        if let Ok(mut config) = state.config.lock() {
            config.rules.insert(ext, destination);
            let _ = config.save(&state.config_path);
        }
    }

    state
        .queue
        .lock()
        .unwrap()
        .retain(|item| !moved_paths.contains(&item.path));
    emit_queue(&app);

    Ok(outcomes)
}

fn record_history(app: &AppHandle, outcome: &MoveOutcome, rule: Option<String>) {
    let entry = HistoryEntry {
        id: Uuid::new_v4().to_string(),
        source: outcome.source.clone(),
        destination: outcome.destination.clone(),
        archived: outcome.archived.clone(),
        rule,
        timestamp: chrono::Local::now().to_rfc3339(),
        undone: false,
    };
    {
        let state = app.state::<AppState>();
        let mut history = state.history.lock().unwrap();
        history.insert(0, entry);
        history.truncate(200);
    }
    emit_history(app);
}

#[tauri::command]
fn scan_downloads(state: State<AppState>) -> Result<Vec<OrganizerItem>, String> {
    let config = state.config.lock().unwrap().clone();
    let watch_dir = config.watch_dir.clone();
    let entries = std::fs::read_dir(&watch_dir)
        .map_err(|err| format!("cannot read {}: {err}", watch_dir.display()))?;

    let mut items = Vec::new();
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
        let suggested = resolve_destination(&config, &ext);
        items.push(OrganizerItem {
            path,
            file_name: name,
            ext,
            size: metadata.len(),
            suggested,
        });
    }

    items.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
    Ok(items)
}

#[tauri::command]
fn organize_files(
    app: AppHandle,
    state: State<AppState>,
    items: Vec<OrganizeRequest>,
) -> Result<Vec<MoveOutcome>, String> {
    let config = state.config.lock().unwrap().clone();
    let watch_dir = config.watch_dir.clone();

    let mut outcomes = Vec::new();
    let mut moved_paths: HashSet<PathBuf> = HashSet::new();
    let mut rules_to_add: Vec<(String, String)> = Vec::new();

    for item in &items {
        let source = PathBuf::from(&item.path);
        let destination_dir = match mover::resolve_destination_path(
            &watch_dir,
            &item.destination,
            config.allow_external_destinations,
        ) {
            Ok(dir) => dir,
            Err(err) => {
                let _ = app.emit("move:failed", format!("{err}: {}", source.display()));
                continue;
            }
        };

        match mover::move_file(
            &source,
            &destination_dir,
            &config.duplicate_policy,
            config.version_timestamp_prefix,
        ) {
            Ok(outcome) => {
                if outcome.moved {
                    moved_paths.insert(source.clone());
                }
                if item.remember {
                    let name = source
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    let ext = classifier::extension_of(&name);
                    if !ext.is_empty() {
                        rules_to_add.push((ext, item.destination.clone()));
                    }
                }
                record_history(
                    &app,
                    &outcome,
                    item.remember.then(|| item.destination.clone()),
                );
                outcomes.push(outcome);
            }
            Err(err) => {
                let _ = app.emit("move:failed", format!("{err}: {}", source.display()));
            }
        }
    }

    if !rules_to_add.is_empty() {
        if let Ok(mut config) = state.config.lock() {
            for (ext, destination) in rules_to_add {
                config.rules.insert(ext, destination);
            }
            let _ = config.save(&state.config_path);
        }
        let _ = app.emit("config:updated", ());
    }

    if !moved_paths.is_empty() {
        state
            .queue
            .lock()
            .unwrap()
            .retain(|item| !moved_paths.contains(&item.path));
        emit_queue(&app);
    }

    Ok(outcomes)
}

#[tauri::command]
fn undo(app: AppHandle, state: State<AppState>, id: String) -> Result<HistoryEntry, String> {
    let mut history = state.history.lock().unwrap();
    let entry = history
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "history entry not found".to_string())?;
    if entry.undone {
        return Err("move was already undone".to_string());
    }
    if !entry.destination.exists() {
        return Err("destination no longer exists".to_string());
    }
    if entry.source.exists() {
        return Err("original location is occupied".to_string());
    }
    if let Some(parent) = entry.source.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::rename(&entry.destination, &entry.source).map_err(|e| format!("undo failed: {e}"))?;
    if let Some(archived) = entry.archived.clone() {
        if archived.exists() {
            let _ = std::fs::rename(&archived, &entry.destination);
        }
    }
    entry.undone = true;
    let updated = entry.clone();
    drop(history);
    emit_history(&app);
    Ok(updated)
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn set_paused_until(app: &AppHandle, until: Option<i64>) {
    let state = app.state::<AppState>();
    if let Ok(mut config) = state.config.lock() {
        config.paused_until = until;
        let _ = config.save(&state.config_path);
    }
    let _ = app.emit("paused:updated", ());
}

fn handle_tray_menu(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "open" => show_main(app),
        "settings" => {
            show_main(app);
            let _ = app.emit("navigate", "settings");
        }
        "recent" => {
            show_main(app);
            let _ = app.emit("navigate", "history");
        }
        "resume" => set_paused_until(app, None),
        "pause_30" => set_paused_until(app, Some(now_ts() + 30 * 60)),
        "pause_60" => set_paused_until(app, Some(now_ts() + 60 * 60)),
        "pause_180" => set_paused_until(app, Some(now_ts() + 3 * 60 * 60)),
        "pause_indef" => set_paused_until(app, Some(i64::MAX)),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let open = MenuItem::with_id(app, "open", "Open Mobius", true, None::<&str>)?;
    let pause_30 = MenuItem::with_id(app, "pause_30", "For 30 minutes", true, None::<&str>)?;
    let pause_60 = MenuItem::with_id(app, "pause_60", "For 1 hour", true, None::<&str>)?;
    let pause_180 = MenuItem::with_id(app, "pause_180", "For 3 hours", true, None::<&str>)?;
    let pause_indef = MenuItem::with_id(app, "pause_indef", "Until resumed", true, None::<&str>)?;
    let pause = Submenu::with_items(
        app,
        "Pause",
        true,
        &[&pause_30, &pause_60, &pause_180, &pause_indef],
    )?;
    let resume = MenuItem::with_id(app, "resume", "Resume", true, None::<&str>)?;
    let recent = MenuItem::with_id(app, "recent", "Recent moves", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Mobius", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &pause,
            &resume,
            &PredefinedMenuItem::separator(app)?,
            &recent,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("Mobius — automatic download organizer")
        .on_menu_event(handle_tray_menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)
}

fn build_menu(app: &tauri::App) -> tauri::Result<Menu<tauri::Wry>> {
    #[cfg(target_os = "macos")]
    let app_menu = Submenu::with_items(
        app,
        "Mobius",
        true,
        &[
            &PredefinedMenuItem::about(
                app,
                Some("About Mobius"),
                Some(tauri::menu::AboutMetadata {
                    name: Some("Mobius".to_string()),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    comments: Some("Automatic download organizer".to_string()),
                    authors: Some(vec!["Dexter Santucci".to_string()]),
                    credits: Some("by Dexter Santucci".to_string()),
                    copyright: Some("© 2026 Dexter Santucci".to_string()),
                    website: Some("https://github.com/DexterLagan/Mobius".to_string()),
                    website_label: Some("GitHub".to_string()),
                    ..Default::default()
                }),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("Quit Mobius"))?,
        ],
    )?;

    #[cfg(not(target_os = "macos"))]
    let app_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &tauri::menu::MenuItem::with_id(app, "about", "About Mobius", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("Quit Mobius"))?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    Menu::with_items(app, &[&app_menu, &edit_menu])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("another instance was launched; focusing the existing window");
            show_main(app);
            use tauri_plugin_dialog::DialogExt;
            app.dialog()
                .message("Mobius is already running. Its window has been brought to the front.")
                .title("Mobius is already running")
                .show(|_| {});
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("config.json");
            let config = Config::load(&config_path);
            if !config_path.exists() {
                let _ = config.save(&config_path);
            }
            let _ = std::fs::create_dir_all(&config.watch_dir);
            let state = AppState {
                config: Mutex::new(config),
                config_path,
                queue: Mutex::new(Vec::new()),
                seen: Mutex::new(HashSet::new()),
                history: Mutex::new(Vec::new()),
                app_start: SystemTime::now(),
                last_scan: Mutex::new(None),
            };
            app.manage(state);

            app.set_menu(build_menu(app)?)?;
            let tray = build_tray(&handle)?;
            app.manage(tray);

            let scanner_handle = handle.clone();
            std::thread::spawn(move || scanner::run(scanner_handle));

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .on_menu_event(|_app, event| {
            #[cfg(not(target_os = "macos"))]
            {
                if event.id().as_ref() == "about" {
                    use tauri_plugin_dialog::DialogExt;
                    _app.dialog()
                        .message(format!(
                            "Mobius {}\nAutomatic download organizer\n\nby Dexter Santucci\nhttps://github.com/DexterLagan/Mobius",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .title("About Mobius")
                        .show(|_| {});
                }
            }
            #[cfg(target_os = "macos")]
            {
                let _ = event;
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            list_queue,
            list_history,
            get_status,
            confirm_move,
            dismiss,
            undo,
            scan_downloads,
            organize_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
